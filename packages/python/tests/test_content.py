"""Opt-in content checks: three-state verified/failed/unverifiable (spec §7)."""

from __future__ import annotations

import base64
import hashlib
import json
from pathlib import Path
from typing import Any

import pytest

from openakb_validate import FullReport, validate_with_content
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


class RecordingResolver(FakeResolver):
    """FakeResolver variant that records every requested URI."""

    def __init__(self, files: dict[str, bytes]) -> None:
        super().__init__(files)
        self.requests: list[str] = []

    def fetch(self, uri: str) -> bytes:
        self.requests.append(uri)
        return super().fetch(uri)


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


def _sidecar(section_id: str = "root", **overrides: object) -> bytes:
    sidecar: dict[str, Any] = {
        "$schema": "https://schema.openakb.org/v1/provenance.schema.json",
        "section_id": section_id,
        "claims": [{"text": "Claim.", "source_ids": ["s1"]}],
    }
    sidecar.update(overrides)
    return json_bytes(sidecar)


def json_bytes(value: object) -> bytes:
    return json.dumps(value, separators=(",", ":")).encode("utf-8")


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


def test_non_markdown_no_fetch() -> None:
    """Non-Markdown sections with no content_hash do not fetch content."""
    section = _descriptor()["sections"][0] | {"content_type": "application/json"}
    resolver = RecordingResolver({"root.md": b"[cite: ghost]"})
    report = check_content(_descriptor(sections=[section]), resolver)

    assert _checks_by_kind(report, "citations") == []
    assert _checks_by_kind(report, "content-hash") == []
    assert resolver.requests == []
    assert report.ok


def test_unknown_hash_no_fetch() -> None:
    """Unsupported non-Markdown content_hash values do not fetch content."""
    section = _descriptor()["sections"][0] | {
        "content_hash": "sha512-aa==",
        "content_type": "application/json",
    }
    resolver = RecordingResolver({"root.md": b"{}"})
    report = check_content(_descriptor(sections=[section]), resolver)

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert resolver.requests == []
    assert report.ok


def test_malformed_hash_no_fetch() -> None:
    """Malformed non-Markdown content_hash values do not fetch content."""
    section = _descriptor()["sections"][0] | {
        "content_hash": "sha256-not!base64",
        "content_type": "application/json",
    }
    resolver = RecordingResolver({"root.md": b"{}"})
    report = check_content(_descriptor(sections=[section]), resolver)

    assert _checks_by_kind(report, "content-hash")[0].outcome == UNVERIFIABLE
    assert len(report.warnings) == 1
    assert resolver.requests == []
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


def test_malformed_type_no_fetch() -> None:
    """Malformed content_type sections with no content_hash do not fetch content."""
    section = _descriptor()["sections"][0] | {"content_type": 42}
    resolver = RecordingResolver({"root.md": b"[cite: ghost]"})
    report = check_content(_descriptor(sections=[section]), resolver)

    assert _checks_by_kind(report, "citations") == []
    assert _checks_by_kind(report, "content-hash") == []
    assert resolver.requests == []
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


def test_local_query_fragment(tmp_path: Path) -> None:
    """Local content aliases with empty queries stay unverifiable through check_content."""
    (tmp_path / "root.md").write_bytes(b"See [cite: s1].")
    section = _descriptor()["sections"][0] | {"content_uri": "root.md?#frag"}
    report = check_content(_descriptor(sections=[section]), LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_local_param_fragment(tmp_path: Path) -> None:
    """Local content aliases with path params stay unverifiable through check_content."""
    (tmp_path / "root.md").write_bytes(b"See [cite: s1].")
    section = _descriptor()["sections"][0] | {"content_uri": "root.md;#frag"}
    report = check_content(_descriptor(sections=[section]), LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_local_guide_query(tmp_path: Path) -> None:
    """Local guide aliases with empty queries stay unverifiable through check_content."""
    guide = b"# Guide\n"
    (tmp_path / "AKB.md").write_bytes(guide)
    descriptor = _descriptor(guide_uri="AKB.md?#intro", guide_hash=_sri(guide))
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_base_query_fragment(tmp_path: Path) -> None:
    """Fragment-only local content refs must not hide base_uri empty query aliases."""
    (tmp_path / "root.md").write_bytes(b"See [cite: s1].")
    section = _descriptor()["sections"][0] | {"content_uri": "#new"}
    descriptor = _descriptor(base_uri="root.md?#old", sections=[section])
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_base_param_fragment(tmp_path: Path) -> None:
    """Fragment-only local content refs must not hide base_uri path parameters."""
    (tmp_path / "root.md").write_bytes(b"See [cite: s1].")
    section = _descriptor()["sections"][0] | {"content_uri": "#new"}
    descriptor = _descriptor(base_uri="root.md;#old", sections=[section])
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_base_guide_query(tmp_path: Path) -> None:
    """Fragment-only local guide refs must not hide base_uri empty query aliases."""
    guide = b"# Guide\n"
    (tmp_path / "AKB.md").write_bytes(guide)
    descriptor = _descriptor(
        base_uri="AKB.md?#old",
        guide_uri="#intro",
        guide_hash=_sri(guide),
    )
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_join_traversal_content(tmp_path: Path) -> None:
    """Local content refs reject literal traversal before urljoin can normalize it."""
    (tmp_path / "root.md").write_bytes(b"See [cite: s1].")
    section = _descriptor()["sections"][0] | {"content_uri": "../root.md"}
    descriptor = _descriptor(base_uri="dir/base.md", sections=[section])
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_join_traversal_guide(tmp_path: Path) -> None:
    """Local guide refs reject literal traversal before urljoin can normalize it."""
    guide = b"# Guide\n"
    (tmp_path / "AKB.md").write_bytes(guide)
    descriptor = _descriptor(
        base_uri="dir/base.md",
        guide_uri="../AKB.md",
        guide_hash=_sri(guide),
    )
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_base_traversal_content(tmp_path: Path) -> None:
    """Local base_uri traversal is rejected before relative content references join."""
    (tmp_path / "root.md").write_bytes(b"See [cite: s1].")
    descriptor = _descriptor(base_uri="dir/../base.md")
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_base_traversal_guide(tmp_path: Path) -> None:
    """Local base_uri traversal is rejected before relative guide references join."""
    guide = b"# Guide\n"
    (tmp_path / "AKB.md").write_bytes(guide)
    descriptor = _descriptor(
        base_uri="dir/../base.md",
        guide_uri="AKB.md",
        guide_hash=_sri(guide),
    )
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_backslash_join_content(tmp_path: Path) -> None:
    """Local content refs reject raw backslashes before platform path handling."""
    directory = tmp_path / "dir"
    directory.mkdir()
    (directory / "..\\root.md").write_bytes(b"See [cite: s1].")
    section = _descriptor()["sections"][0] | {"content_uri": "..\\root.md"}
    descriptor = _descriptor(base_uri="dir/base.md", sections=[section])
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "citations")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_backslash_join_guide(tmp_path: Path) -> None:
    """Local guide refs reject raw backslashes before platform path handling."""
    guide = b"# Guide\n"
    directory = tmp_path / "dir"
    directory.mkdir()
    (directory / "..\\AKB.md").write_bytes(guide)
    descriptor = _descriptor(
        base_uri="dir/base.md",
        guide_uri="..\\AKB.md",
        guide_hash=_sri(guide),
    )
    report = check_content(descriptor, LocalFileResolver(tmp_path))

    assert _checks_by_kind(report, "guide-hash")[0].outcome == UNVERIFIABLE
    assert report.ok


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


def test_backslash_traversal_rejected(tmp_path: Path) -> None:
    """Raw backslashes are rejected before platform path handling can traverse."""
    base = tmp_path / "kb"
    base.mkdir()
    (base / "dir\\..\\root.md").write_bytes(b"content")

    with pytest.raises(Unfetchable):
        LocalFileResolver(base).fetch("dir\\..\\root.md")


def test_sidecar_bound_verifies() -> None:
    """A conforming sidecar bound to its descriptor section verifies."""
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": _sidecar()}),
    )

    assert _checks_by_kind(report, "sidecar")[0].outcome == VERIFIED
    assert report.ok


def test_sidecar_hash_verified() -> None:
    """A provenance_hash verifies against fetched sidecar bytes."""
    payload = _sidecar()
    section = _descriptor()["sections"][0] | {
        "provenance_uri": "root.prov.json",
        "provenance_hash": _sri(payload),
    }
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": payload}),
    )

    assert [check.outcome for check in _checks_by_kind(report, "sidecar")] == [
        VERIFIED,
        VERIFIED,
    ]


def test_sidecar_unparseable_fails() -> None:
    """Sidecar bytes that are not JSON fail content verification."""
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": b"not json"}),
    )

    assert _checks_by_kind(report, "sidecar")[0].outcome == FAILED
    assert not report.ok


def test_sidecar_schema_codes() -> None:
    """Sidecar schema findings keep the standard schema-layer codes."""
    payload = _sidecar(claims=[{"source_ids": ["Bad ID"]}])
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": payload}),
    )

    assert _checks_by_kind(report, "sidecar")[0].outcome == FAILED
    assert {finding.code for finding in report.findings} == {"AKB009", "AKB011"}


def test_sidecar_schema_path() -> None:
    """Sidecar schema finding paths are anchored at descriptor provenance_uri."""
    payload = _sidecar(claims=[{"text": "Claim.", "source_ids": ["Bad ID"]}])
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": payload}),
    )

    sidecar = _checks_by_kind(report, "sidecar")[0]
    assert [finding.path for finding in sidecar.findings] == [
        "/sections/0/provenance_uri/claims/0/source_ids/0"
    ]


def test_sidecar_binding_mismatch() -> None:
    """A sidecar bound to another section fails without inventing a code."""
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(
            sections=[
                section,
                {
                    "id": "other",
                    "title": "Other",
                    "description": "Other section.",
                    "source_ids": ["s1"],
                },
            ]
        ),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": _sidecar("other")}),
    )

    sidecar = _checks_by_kind(report, "sidecar")[0]
    assert sidecar.outcome == FAILED
    assert sidecar.findings == ()


def test_sidecar_mismatch_detail() -> None:
    """Binding mismatch detail names the section_id mismatch."""
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(
            sections=[
                section,
                {
                    "id": "other",
                    "title": "Other",
                    "description": "Other section.",
                    "source_ids": ["s1"],
                },
            ]
        ),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": _sidecar("other")}),
    )

    assert "section_id" in _checks_by_kind(report, "sidecar")[0].detail


def test_sidecar_section_akb007() -> None:
    """A sidecar section_id that resolves nowhere emits AKB007."""
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": _sidecar("ghost")}),
    )

    assert [finding.code for finding in report.findings] == ["AKB007"]


def test_sidecar_claim_akb010() -> None:
    """A sidecar claim source_id that names a section emits AKB010."""
    payload = _sidecar(claims=[{"text": "Claim.", "source_ids": ["root"]}])
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "root.prov.json": payload}),
    )

    assert [finding.code for finding in report.findings] == ["AKB010"]


def test_sidecar_unfetchable() -> None:
    """An unavailable provenance sidecar is unverifiable."""
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sections=[section]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    assert _checks_by_kind(report, "sidecar")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_capture_hash_verified() -> None:
    """A source content_hash verifies against fetched capture bytes."""
    capture = b"Captured text."
    source = _descriptor()["sources"][0] | {
        "capture_uri": "capture.bin",
        "content_hash": _sri(capture),
    }
    report = check_content(
        _descriptor(sources=[source]),
        FakeResolver({"root.md": b"See [cite: s1].", "capture.bin": capture}),
    )

    assert _checks_by_kind(report, "capture")[0].outcome == VERIFIED


def test_capture_hash_failed() -> None:
    """A source content_hash mismatch fails against fetched capture bytes."""
    source = _descriptor()["sources"][0] | {
        "capture_uri": "capture.bin",
        "content_hash": _sri(b"expected"),
    }
    report = check_content(
        _descriptor(sources=[source]),
        FakeResolver({"root.md": b"See [cite: s1].", "capture.bin": b"actual"}),
    )

    assert _checks_by_kind(report, "capture")[0].outcome == FAILED
    assert not report.ok


def test_capture_hashless_unverifiable() -> None:
    """A source content_hash without capture_uri is unverifiable."""
    source = _descriptor()["sources"][0] | {"content_hash": _sri(b"expected")}
    report = check_content(
        _descriptor(sources=[source]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    assert _checks_by_kind(report, "capture")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_capture_algo_warns() -> None:
    """Unsupported source content_hash without capture_uri preserves SRI warning."""
    source = _descriptor()["sources"][0] | {"content_hash": "sha512-aa=="}
    report = check_content(
        _descriptor(sources=[source]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    capture = _checks_by_kind(report, "capture")[0]
    assert capture.outcome == UNVERIFIABLE
    assert "unsupported hash algorithm" in capture.detail
    assert len(capture.warnings) == 1
    assert report.ok


def test_capture_malformed_warns() -> None:
    """Malformed source content_hash without capture_uri preserves SRI warning."""
    source = _descriptor()["sources"][0] | {"content_hash": "sha256-not!base64"}
    report = check_content(
        _descriptor(sources=[source]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    capture = _checks_by_kind(report, "capture")[0]
    assert capture.outcome == UNVERIFIABLE
    assert "malformed sha256 digest" in capture.detail
    assert len(capture.warnings) == 1
    assert report.ok


def test_capture_uri_fetches() -> None:
    """Non-redacted capture_uri fetches even without hash or quote usage."""
    source = _descriptor()["sources"][0] | {"capture_uri": "capture.bin"}
    resolver = RecordingResolver({"root.md": b"See [cite: s1].", "capture.bin": b"capture"})
    report = check_content(_descriptor(sources=[source]), resolver)

    assert resolver.requests == ["root.md", "capture.bin"]
    assert _checks_by_kind(report, "capture") == []
    assert report.ok


def test_capture_malformed_fetches() -> None:
    """Malformed source content_hash does not prevent capture_uri fetch."""
    source = _descriptor()["sources"][0] | {
        "capture_uri": "capture.bin",
        "content_hash": "sha256-not!base64",
    }
    resolver = RecordingResolver({"root.md": b"See [cite: s1].", "capture.bin": b"capture"})
    report = check_content(_descriptor(sources=[source]), resolver)

    capture = _checks_by_kind(report, "capture")[0]
    assert resolver.requests == ["root.md", "capture.bin"]
    assert capture.outcome == UNVERIFIABLE
    assert "malformed sha256 digest" in capture.detail
    assert len(capture.warnings) == 1
    assert report.ok


def test_capture_unfetchable_reports() -> None:
    """Hashless capture_uri fetch failures emit an unverifiable capture check."""
    source = _descriptor()["sources"][0] | {"capture_uri": "missing.bin"}
    report = check_content(
        _descriptor(sources=[source]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    capture = _checks_by_kind(report, "capture")[0]
    assert capture.outcome == UNVERIFIABLE
    assert capture.path == "/sources/0/capture_uri"
    assert report.ok


def test_capture_malformed_unfetchable() -> None:
    """Malformed hashes do not hide unfetchable capture_uri diagnostics."""
    source = _descriptor()["sources"][0] | {
        "capture_uri": "missing.bin",
        "content_hash": "sha256-not!base64",
    }
    report = check_content(
        _descriptor(sources=[source]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    captures = _checks_by_kind(report, "capture")
    assert [check.path for check in captures] == [
        "/sources/0/content_hash",
        "/sources/0/capture_uri",
    ]
    assert [check.outcome for check in captures] == [UNVERIFIABLE, UNVERIFIABLE]
    assert len(captures[0].warnings) == 1
    assert report.ok


def test_quote_found_verifies() -> None:
    """Inline quote provenance verifies against any cited fetched capture."""
    source = _descriptor()["sources"][0] | {"capture_uri": "capture.bin"}
    section = _descriptor()["sections"][0] | {
        "provenance": [{"text": "Claim.", "source_ids": ["s1"], "locator": {"quote": "needle"}}]
    }
    report = check_content(
        _descriptor(sources=[source], sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "capture.bin": b"hay needle stack"}),
    )

    assert _checks_by_kind(report, "quote")[0].outcome == VERIFIED


def test_quote_absent_fails() -> None:
    """Inline quote provenance fails when fetched captures lack the quote."""
    source = _descriptor()["sources"][0] | {"capture_uri": "capture.bin"}
    section = _descriptor()["sections"][0] | {
        "provenance": [{"text": "Claim.", "source_ids": ["s1"], "locator": {"quote": "needle"}}]
    }
    report = check_content(
        _descriptor(sources=[source], sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "capture.bin": b"hay stack"}),
    )

    assert _checks_by_kind(report, "quote")[0].outcome == FAILED
    assert not report.ok


def test_quote_partial_unfetchable() -> None:
    """Quote absence is unverifiable when another cited capture cannot fetch."""
    sources = [
        _descriptor()["sources"][0] | {"capture_uri": "s1.bin"},
        {"id": "s2", "type": "url", "uri": "https://other.example.com/", "capture_uri": "s2.bin"},
    ]
    section = _descriptor()["sections"][0] | {
        "provenance": [
            {"text": "Claim.", "source_ids": ["s1", "s2"], "locator": {"quote": "needle"}}
        ]
    }
    report = check_content(
        _descriptor(sources=sources, sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "s1.bin": b"hay stack"}),
    )

    assert _checks_by_kind(report, "quote")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_quote_partial_redacted() -> None:
    """Quote absence is unverifiable when another cited source is redacted."""
    sources = [
        _descriptor()["sources"][0] | {"capture_uri": "s1.bin"},
        {"id": "s2", "type": "redacted"},
    ]
    section = _descriptor()["sections"][0] | {
        "provenance": [
            {"text": "Claim.", "source_ids": ["s1", "s2"], "locator": {"quote": "needle"}}
        ]
    }
    report = check_content(
        _descriptor(sources=sources, sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1].", "s1.bin": b"hay stack"}),
    )

    assert _checks_by_kind(report, "quote")[0].outcome == UNVERIFIABLE
    assert "redacted" in report.warnings[0].message
    assert report.ok


def test_quote_all_absent_fails() -> None:
    """Quote absence fails when all cited captures are fetched and lack it."""
    sources = [
        _descriptor()["sources"][0] | {"capture_uri": "s1.bin"},
        {"id": "s2", "type": "url", "uri": "https://other.example.com/", "capture_uri": "s2.bin"},
    ]
    section = _descriptor()["sections"][0] | {
        "provenance": [
            {"text": "Claim.", "source_ids": ["s1", "s2"], "locator": {"quote": "needle"}}
        ]
    }
    report = check_content(
        _descriptor(sources=sources, sections=[section]),
        FakeResolver(
            {
                "root.md": b"See [cite: s1].",
                "s1.bin": b"hay stack",
                "s2.bin": b"other hay stack",
            }
        ),
    )

    assert _checks_by_kind(report, "quote")[0].outcome == FAILED
    assert not report.ok


def test_quote_without_capture() -> None:
    """Inline quote provenance is unverifiable without fetched captures."""
    section = _descriptor()["sections"][0] | {
        "provenance": [{"text": "Claim.", "source_ids": ["s1"], "locator": {"quote": "needle"}}]
    }
    report = check_content(
        _descriptor(sections=[section]), FakeResolver({"root.md": b"See [cite: s1]."})
    )

    assert _checks_by_kind(report, "quote")[0].outcome == UNVERIFIABLE
    assert report.ok


def test_sidecar_quote_verifies() -> None:
    """Sidecar claim quotes join quote verification inputs."""
    source = _descriptor()["sources"][0] | {"capture_uri": "capture.bin"}
    payload = _sidecar(
        claims=[{"text": "Claim.", "source_ids": ["s1"], "locator": {"quote": "needle"}}]
    )
    section = _descriptor()["sections"][0] | {"provenance_uri": "root.prov.json"}
    report = check_content(
        _descriptor(sources=[source], sections=[section]),
        FakeResolver(
            {
                "root.md": b"See [cite: s1].",
                "capture.bin": b"hay needle stack",
                "root.prov.json": payload,
            }
        ),
    )

    assert _checks_by_kind(report, "sidecar")[0].outcome == VERIFIED
    assert _checks_by_kind(report, "quote")[0].outcome == VERIFIED


def test_redacted_quote_warns() -> None:
    """Quotes citing only redacted sources are unverifiable with an advisory."""
    source = {"id": "s1", "type": "redacted"}
    section = _descriptor()["sections"][0] | {
        "provenance": [{"text": "Claim.", "source_ids": ["s1"], "locator": {"quote": "needle"}}]
    }
    report = check_content(
        _descriptor(sources=[source], sections=[section]),
        FakeResolver({"root.md": b"See [cite: s1]."}),
    )

    assert _checks_by_kind(report, "quote")[0].outcome == UNVERIFIABLE
    assert "redacted" in report.warnings[0].message
    assert report.ok


def test_redacted_hash_unverifiable() -> None:
    """Redacted sources never fetch and make source hashes unverifiable."""
    source = {
        "id": "s1",
        "type": "redacted",
        "capture_uri": "capture.bin",
        "content_hash": _sri(b"capture"),
    }
    resolver = RecordingResolver({"root.md": b"See [cite: s1].", "capture.bin": b"capture"})
    report = check_content(_descriptor(sources=[source]), resolver)

    assert _checks_by_kind(report, "capture")[0].outcome == UNVERIFIABLE
    assert resolver.requests == ["root.md"]
    assert report.ok


def test_redacted_capture_ignored() -> None:
    """Redacted sources do not contribute capture bytes even with capture_uri."""
    source = {"id": "s1", "type": "redacted", "capture_uri": "capture.bin"}
    section = _descriptor()["sections"][0] | {
        "provenance": [{"text": "Claim.", "source_ids": ["s1"], "locator": {"quote": "needle"}}]
    }
    resolver = RecordingResolver({"root.md": b"See [cite: s1].", "capture.bin": b"needle"})
    report = check_content(_descriptor(sources=[source], sections=[section]), resolver)

    assert _checks_by_kind(report, "capture")[0].outcome == UNVERIFIABLE
    assert _checks_by_kind(report, "quote")[0].outcome == UNVERIFIABLE
    assert resolver.requests == ["root.md"]


def test_full_report_combines() -> None:
    """The validate_with_content facade returns both reports and combined ok."""
    source = _descriptor()["sources"][0] | {"capture_uri": "capture.bin"}
    report = validate_with_content(
        _descriptor(sources=[source]),
        FakeResolver({"root.md": b"See [cite: s1].", "capture.bin": b"capture"}),
    )

    assert isinstance(report, FullReport)
    assert report.validation.ok
    assert report.content.ok
    assert report.ok


def test_full_content_failure() -> None:
    """A content failure flips FullReport.ok."""
    source = _descriptor()["sources"][0] | {
        "capture_uri": "capture.bin",
        "content_hash": _sri(b"expected"),
    }
    report = validate_with_content(
        _descriptor(sources=[source]),
        FakeResolver({"root.md": b"See [cite: s1].", "capture.bin": b"actual"}),
    )

    assert report.validation.ok
    assert not report.content.ok
    assert not report.ok


def test_full_structural_failure() -> None:
    """A structural validation failure flips FullReport.ok."""
    descriptor = _descriptor(title=7)
    report = validate_with_content(descriptor, FakeResolver({"root.md": b"See [cite: s1]."}))

    assert not report.validation.ok
    assert report.content.ok
    assert not report.ok


def test_nonmarkdown_hash_fetches() -> None:
    """Valid non-Markdown content_hash values still fetch and verify bytes."""
    payload = b'{"ok":true}'
    section = _descriptor()["sections"][0] | {
        "content_type": "application/json",
        "content_hash": _sri(payload),
    }
    resolver = RecordingResolver({"root.md": payload})
    report = check_content(_descriptor(sections=[section]), resolver)

    assert _checks_by_kind(report, "content-hash")[0].outcome == VERIFIED
    assert resolver.requests == ["root.md"]


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
