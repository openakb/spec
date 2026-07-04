"""Keyword->code mapping over the bundled published schema (spec §7)."""

from typing import Any

import pytest

from openakb_validate.schema import _ecma_anchor, provenance_validator, schema_findings

__all__ = ()


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


def _codes(instance: object) -> set[str]:
    return {finding.code for finding in schema_findings(instance)}


def test_valid_descriptor_has_no_findings() -> None:
    assert schema_findings(_descriptor()) == []


def test_akb009_missing_required_field() -> None:
    descriptor = _descriptor()
    del descriptor["title"]
    assert "AKB009" in _codes(descriptor)


def test_akb005_over_cap_title() -> None:
    assert "AKB005" in _codes(_descriptor(title="x" * 201))


def test_akb011_bad_id_charset() -> None:
    assert "AKB011" in _codes(_descriptor(id="Bad Id"))


def test_akb011_wrong_schema_uri() -> None:
    codes = _codes(_descriptor(**{"$schema": "https://example.com/other.json"}))
    assert "AKB011" in codes


def test_akb011_invalid_timestamp() -> None:
    """Format checker must assert real calendar dates, not just regex grammar."""
    assert "AKB011" in _codes(_descriptor(created_at="2026-02-30T00:00:00Z"))


def test_akb011_malformed_guide_hash() -> None:
    """Malformed SRI guide_hash maps via pattern to AKB011."""
    assert "AKB011" in _codes(_descriptor(guide_hash="sha256-not!base64"))


def test_akb012_link_without_target() -> None:
    descriptor = _descriptor()
    descriptor["sections"][0]["links"] = [{"rel": "see-also"}]
    codes = _codes(descriptor)
    assert "AKB012" in codes
    assert "AKB009" not in codes


def test_akb008_unknown_rel() -> None:
    descriptor = _descriptor()
    descriptor["sections"][0]["links"] = [{"rel": "not-a-rel", "section_id": "root"}]
    assert "AKB008" in _codes(descriptor)


def test_akb011_non_string_rel() -> None:
    descriptor = _descriptor()
    descriptor["sections"][0]["links"] = [{"rel": 42, "section_id": "root"}]
    codes = _codes(descriptor)
    assert "AKB011" in codes
    assert "AKB008" not in codes


def test_akb003_missing_source_cite() -> None:
    descriptor = _descriptor()
    del descriptor["sections"][0]["source_ids"]
    assert "AKB003" in _codes(descriptor)


def test_akb003_empty_source_ids() -> None:
    descriptor = _descriptor()
    descriptor["sections"][0]["source_ids"] = []
    assert "AKB003" in _codes(descriptor)


def test_akb011_non_object_root() -> None:
    assert "AKB011" in _codes([])


def test_findings_carry_json_pointer_paths() -> None:
    findings = schema_findings(_descriptor(title="x" * 201))
    assert any(f.path == "/title" for f in findings)


def test_akb011_trailing_newline_id() -> None:
    """A trailing newline breaks the local-id pattern under ECMA-262 `$` semantics."""
    descriptor = _descriptor(
        sources=[{"id": "s1\n", "type": "url", "uri": "https://docs.example.com/"}],
        sections=[
            {
                "id": "root",
                "title": "Root",
                "description": "Root section.",
                "content_uri": "root.md",
                "source_ids": ["s1\n"],
            }
        ],
    )
    findings = schema_findings(descriptor)
    assert "AKB011" in {finding.code for finding in findings}
    assert any(finding.path == "/sources/0/id" for finding in findings)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("namespace", "abc\n"),
        ("language", "en\n"),
        ("guide_hash", "sha256-YQ==\n"),
    ],
)
def test_akb011_trailing_newline_fields(field: str, value: str) -> None:
    """Pattern-anchored top-level fields reject trailing newlines like ajv does."""
    assert "AKB011" in _codes(_descriptor(**{field: value}))


def test_akb011_trailing_newline_tag() -> None:
    """Trailing newlines in tag items are rejected (array items carry the pattern)."""
    assert "AKB011" in _codes(_descriptor(tags=["abc\n"]))


def test_akb011_trailing_newline_x_key() -> None:
    """propertyNames pattern rejects trailing-newline extension keys (AKB011)."""
    assert "AKB011" in _codes(_descriptor(x={"a.b\n": {}}))


def test_akb008_trailing_newline_rel() -> None:
    """A trailing newline breaks the reverse-DNS rel escape, yielding AKB008."""
    descriptor = _descriptor()
    descriptor["sections"][0]["links"] = [{"rel": "a.b:c\n", "section_id": "root"}]
    assert "AKB008" in _codes(descriptor)


def test_pattern_matches_without_trailing_newline() -> None:
    """Well-formed values still validate: the ECMA anchoring is not over-broad."""
    assert schema_findings(_descriptor(namespace="abc", tags=["ok"])) == []


@pytest.mark.parametrize(
    ("pattern", "expected"),
    [
        ("^[a-z0-9_-]+$", r"^[a-z0-9_-]+\Z"),
        ("a$", r"a\Z"),
        (r"a\$b", r"a\$b"),
        ("[a$]", "[a$]"),
        ("[]$]", "[]$]"),
        ("[^]]$", r"[^]]\Z"),
        ("no-anchor", "no-anchor"),
    ],
)
def test_ecma_anchor_translation(pattern: str, expected: str) -> None:
    """`$` anchors become `\\Z`; escaped or in-class dollars stay literal."""
    assert _ecma_anchor(pattern) == expected


def test_provenance_validator_is_usable() -> None:
    provenance = {
        "$schema": "https://schema.openakb.org/v1/provenance.schema.json",
        "section_id": "root",
        "claims": [{"text": "A claim.", "source_ids": ["s1"]}],
    }
    assert schema_findings(provenance, provenance_validator()) == []
