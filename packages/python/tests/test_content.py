"""Opt-in content checks: three-state verified/failed/unverifiable (spec §7)."""

from __future__ import annotations

import base64
import hashlib
from pathlib import Path
from typing import Any

import pytest

from openakb_validate.content import (
    FAILED,
    UNVERIFIABLE,
    VERIFIED,
    ContentCheck,
    ContentReport,
    LocalFileResolver,
    Unfetchable,
    check_content,
)

__all__ = ()

_BASE64_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"


class FakeResolver:
    """Maps exact reference strings to bytes; everything else is Unfetchable."""

    def __init__(self, files: dict[str, bytes]) -> None:
        self.files = files

    def fetch(self, uri: str) -> bytes:
        if uri not in self.files:
            raise Unfetchable(f"not available: {uri}")
        return self.files[uri]


def _sri(payload: bytes) -> str:
    return "sha256-" + base64.b64encode(hashlib.sha256(payload).digest()).decode("ascii")


def _noncanonical_base64(encoded: str) -> str:
    pad_index = encoded.index("=") - 1
    value = _BASE64_ALPHABET.index(encoded[pad_index])
    replacement = _BASE64_ALPHABET[(value & 0b111100) | ((value + 1) & 0b000011)]
    return encoded[:pad_index] + replacement + encoded[pad_index + 1 :]


def _descriptor(**overrides: object) -> dict[str, Any]:
    base: dict[str, Any] = {
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "kb",
        "title": "KB",
        "description": "A test descriptor.",
        "sources": [{"id": "s1", "type": "url", "uri": "https://docs.example.com/"}],
        "sections": [
            {
                "id": "root",
                "title": "Root",
                "description": "Root section.",
                "content_uri": "root.md",
                "source_ids": ["s1"],
            }
        ],
    }
    base.update(overrides)
    return base


def _checks_by_kind(report: ContentReport, kind: str) -> list[ContentCheck]:
    return [check for check in report.checks if check.kind == kind]


def test_matching_content() -> None:
    """A matching section content hash and resolvable citations both verify."""
    payload = b"See [cite: s1]."
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_hash": _sri(payload)}]
    )
    report = check_content(descriptor, FakeResolver({"root.md": payload}))

    assert {check.kind: check.outcome for check in report.checks} == {
        "content-hash": VERIFIED,
        "citations": VERIFIED,
    }
    assert report.ok


def test_hash_mismatch() -> None:
    """A sha256 digest mismatch is a failed check and flips report.ok."""
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_hash": _sri(b"expected")}]
    )
    report = check_content(descriptor, FakeResolver({"root.md": b"actual"}))

    checks = _checks_by_kind(report, "content-hash")
    assert [check.outcome for check in checks] == [FAILED]
    assert report.failed == tuple(checks)
    assert not report.ok


def test_unknown_hash_algo() -> None:
    """An unsupported hash algorithm is unverifiable, not failed."""
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_hash": "sha512-aa=="}]
    )
    report = check_content(descriptor, FakeResolver({"root.md": b"anything"}))

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert report.ok


def test_unknown_unfetchable_hash() -> None:
    """Unsupported content_hash algorithms warn even if content cannot be fetched."""
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_hash": "sha512-aa=="}]
    )
    report = check_content(descriptor, FakeResolver({}))

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert report.ok


def test_malformed_sha256_hash() -> None:
    """Malformed sha256 base64 is unverifiable because schema owns shape errors."""
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_hash": "sha256-not!base64"}]
    )
    report = check_content(descriptor, FakeResolver({"root.md": b"anything"}))

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert report.ok


def test_malformed_unfetchable_guide() -> None:
    """Malformed guide_hash values warn even if the guide cannot be fetched."""
    descriptor = _descriptor(guide_uri="missing.md", guide_hash="sha256-not!base64")
    report = check_content(descriptor, FakeResolver({"root.md": b""}))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert report.ok


def test_wrong_digest_length() -> None:
    """A decoded sha256 digest with the wrong byte length is unverifiable."""
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_hash": "sha256-YQ=="}]
    )
    report = check_content(descriptor, FakeResolver({"root.md": b"anything"}))

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert report.ok


def test_noncanonical_base64() -> None:
    """Non-canonical sha256 base64 is unverifiable, not an integrity mismatch."""
    payload = b"anything"
    digest = base64.b64encode(hashlib.sha256(payload).digest()).decode("ascii")
    descriptor = _descriptor(
        sections=[
            _descriptor()["sections"][0]
            | {"content_hash": f"sha256-{_noncanonical_base64(digest)}"}
        ]
    )
    report = check_content(descriptor, FakeResolver({"root.md": payload}))

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert report.ok


def test_guide_hash_verifies() -> None:
    """The top-level guide hash verifies against fetched guide bytes."""
    payload = b"# Guide\n"
    descriptor = _descriptor(guide_uri="AKB.md", guide_hash=_sri(payload))
    report = check_content(descriptor, FakeResolver({"root.md": b"", "AKB.md": payload}))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == VERIFIED


def test_guide_fragment_stripped() -> None:
    """Guide references strip fragments before resolver fetch."""
    payload = b"# Guide\n"
    descriptor = _descriptor(guide_uri="AKB.md#intro", guide_hash=_sri(payload))
    report = check_content(descriptor, FakeResolver({"root.md": b"", "AKB.md": payload}))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == VERIFIED


def test_guide_hash_mismatch() -> None:
    """A mismatched guide_hash is a failed check."""
    descriptor = _descriptor(guide_uri="AKB.md", guide_hash=_sri(b"expected"))
    report = check_content(descriptor, FakeResolver({"root.md": b"", "AKB.md": b"actual"}))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == FAILED
    assert not report.ok


def test_unfetchable_guide() -> None:
    """An unavailable guide is unverifiable and never flips report.ok."""
    descriptor = _descriptor(guide_uri="AKB.md", guide_hash=_sri(b"guide"))
    report = check_content(descriptor, FakeResolver({"root.md": b""}))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_malformed_base_guide() -> None:
    """Malformed base_uri values make guide checks unverifiable, never raised."""
    descriptor = _descriptor(
        base_uri="http://[::1",
        guide_uri="AKB.md",
        guide_hash=_sri(b"guide"),
    )
    report = check_content(descriptor, FakeResolver({}))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_unfetchable_content() -> None:
    """Unavailable section content makes citation checks unverifiable."""
    report = check_content(_descriptor(), FakeResolver({}))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_malformed_base_content() -> None:
    """Malformed base_uri values make content checks unverifiable, never raised."""
    report = check_content(_descriptor(base_uri="http://[::1"), FakeResolver({}))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_content_fragment_stripped() -> None:
    """Section content references strip fragments before resolver fetch."""
    descriptor = _descriptor(
        sections=[_descriptor()["sections"][0] | {"content_uri": "root.md#part"}]
    )
    report = check_content(descriptor, FakeResolver({"root.md": b"See [cite: s1]."}))

    assert _checks_by_kind(report, "citations")[0].outcome == VERIFIED


def test_invalid_utf8_markdown() -> None:
    """Markdown bytes that cannot decode as UTF-8 make citation checks unverifiable."""
    report = check_content(_descriptor(), FakeResolver({"root.md": b"\xff"}))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_unresolved_citation() -> None:
    """Citation IDs that match no source or section emit AKB007."""
    report = check_content(_descriptor(), FakeResolver({"root.md": b"See [cite: ghost]."}))

    assert _checks_by_kind(report, "citations")[0].outcome == FAILED
    assert [finding.code for finding in report.findings] == ["AKB007"]
    assert not report.ok


def test_citation_to_section() -> None:
    """Inline citations must point to source IDs, not section IDs."""
    report = check_content(_descriptor(), FakeResolver({"root.md": b"See [cite: root]."}))

    assert _checks_by_kind(report, "citations")[0].outcome == FAILED
    assert [finding.code for finding in report.findings] == ["AKB010"]


def test_duplicate_marker_ids() -> None:
    """Duplicate IDs within one citation marker warn without failing."""
    report = check_content(_descriptor(), FakeResolver({"root.md": b"See [cite: s1, s1]."}))

    assert _checks_by_kind(report, "citations")[0].outcome == VERIFIED
    assert len(report.warnings) == 1
    assert report.ok


def test_non_markdown_skips() -> None:
    """Non-Markdown section content types skip citation extraction."""
    section = _descriptor()["sections"][0] | {"content_type": "application/json"}
    report = check_content(
        _descriptor(sections=[section]), FakeResolver({"root.md": b"[cite: ghost]"})
    )

    assert _checks_by_kind(report, "citations") == []
    assert report.ok


def test_malformed_type_skips() -> None:
    """Malformed content_type values are schema-owned and skip citation extraction."""
    section = _descriptor()["sections"][0] | {"content_type": 42}
    report = check_content(
        _descriptor(sections=[section]), FakeResolver({"root.md": b"[cite: ghost]"})
    )

    assert _checks_by_kind(report, "citations") == []
    assert report.findings == ()
    assert report.ok


def test_base_uri_prefixes() -> None:
    """Relative content and guide references resolve against descriptor base_uri."""
    payload = b"See [cite: s1]."
    guide = b"# Guide\n"
    descriptor = _descriptor(
        base_uri="https://kb.example.org/root/",
        guide_uri="AKB.md",
        guide_hash=_sri(guide),
        sections=[_descriptor()["sections"][0] | {"content_hash": _sri(payload)}],
    )
    report = check_content(
        descriptor,
        FakeResolver(
            {
                "https://kb.example.org/root/root.md": payload,
                "https://kb.example.org/root/AKB.md": guide,
            }
        ),
    )

    assert {check.kind: check.outcome for check in report.checks} == {
        "guide-hash": VERIFIED,
        "content-hash": VERIFIED,
        "citations": VERIFIED,
    }


def test_local_resolver_confines(tmp_path: Path) -> None:
    """LocalFileResolver fetches base-relative files and rejects hostile references."""
    base = tmp_path / "kb"
    base.mkdir()
    (base / "root.md").write_bytes(b"content")
    resolver = LocalFileResolver(base)

    assert resolver.fetch("root.md") == b"content"
    for uri in (
        "../root.md",
        "/root.md",
        "file:root.md",
        "data:text/plain,root",
        "https://kb.example.org/root.md",
        "root.md?variant=old",
        "root.md?",
        "root.md?#frag",
        "root.md;variant=old",
        "root.md;",
    ):
        with pytest.raises(Unfetchable):
            resolver.fetch(uri)


def test_local_resolver_fragments(tmp_path: Path) -> None:
    """Fragments are client-side identifiers; local bytes still come from the file."""
    base = tmp_path / "kb"
    base.mkdir()
    (base / "root.md").write_bytes(b"content")

    assert LocalFileResolver(base).fetch("root.md#section") == b"content"


def test_segment_params_rejected(tmp_path: Path) -> None:
    """Path parameters on any segment must not alias a local filesystem path."""
    base = tmp_path / "kb"
    directory = base / "dir"
    parameter_directory = base / "dir;v"
    directory.mkdir(parents=True)
    parameter_directory.mkdir()
    (directory / "root.md").write_bytes(b"content")
    (parameter_directory / "root.md").write_bytes(b"parameterized")

    with pytest.raises(Unfetchable):
        LocalFileResolver(base).fetch("dir;v/root.md")


def test_traversal_alias_rejected(tmp_path: Path) -> None:
    """Literal '..' segments are traversal even when normalization stays under base."""
    base = tmp_path / "kb"
    directory = base / "dir"
    directory.mkdir(parents=True)
    (base / "root.md").write_bytes(b"content")

    with pytest.raises(Unfetchable):
        LocalFileResolver(base).fetch("dir/../root.md")


def test_percent_traversal_literal(tmp_path: Path) -> None:
    """Percent-encoded traversal text is a literal local filename, not path traversal."""
    base = tmp_path / "kb"
    base.mkdir()
    (base / "%2e%2e").mkdir()
    (base / "%2e%2e" / "root.md").write_bytes(b"literal")

    assert LocalFileResolver(base).fetch("%2e%2e/root.md") == b"literal"


def test_symlink_escape_rejected(tmp_path: Path) -> None:
    """Symlinks under the base must not resolve to files outside the base."""
    base = tmp_path / "kb"
    outside = tmp_path / "outside"
    base.mkdir()
    outside.mkdir()
    (outside / "root.md").write_bytes(b"escape")
    (base / "link.md").symlink_to(outside / "root.md")

    with pytest.raises(Unfetchable):
        LocalFileResolver(base).fetch("link.md")


def test_malformed_uri_rejected(tmp_path: Path) -> None:
    """Malformed URI references raise Unfetchable from LocalFileResolver."""
    base = tmp_path / "kb"
    base.mkdir()

    with pytest.raises(Unfetchable):
        LocalFileResolver(base).fetch("http://[::1")


def test_symlink_loop_rejected(tmp_path: Path) -> None:
    """Symlink loops under the base raise Unfetchable, not RuntimeError."""
    base = tmp_path / "kb"
    base.mkdir()
    (base / "loop").symlink_to(base / "loop")

    with pytest.raises(Unfetchable):
        LocalFileResolver(base).fetch("loop")


def test_missing_file() -> None:
    """A missing local file raises Unfetchable."""
    resolver = LocalFileResolver(Path("does-not-exist"))

    with pytest.raises(Unfetchable):
        resolver.fetch("root.md")


def test_non_dict_empty() -> None:
    """Non-object descriptors produce an empty content report."""
    report = check_content([], FakeResolver({}))

    assert report.checks == ()
    assert report.ok
