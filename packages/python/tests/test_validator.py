"""The validate() facade: schema + semantic layers, plus strict mode."""

from typing import Any

from openakb_validate import Advisory, Finding, ValidationResult, validate

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


def test_valid_in_both_modes() -> None:
    assert validate(_descriptor()).ok
    assert validate(_descriptor(), strict=True).ok


def test_discovery_cycle_warns_ok() -> None:
    """A source discovery cycle is advisory and does not invalidate the descriptor."""
    descriptor = _descriptor(
        sources=[
            {
                "id": "a",
                "type": "feed",
                "uri": "https://docs.example.com/a",
                "discovered_via_id": "b",
            },
            {
                "id": "b",
                "type": "feed",
                "uri": "https://docs.example.com/b",
                "discovered_via_id": "a",
            },
        ],
        sections=[
            {
                "id": "root",
                "title": "Root",
                "description": "Root section.",
                "content_uri": "root.md",
                "source_ids": ["a"],
            }
        ],
    )

    result = validate(descriptor)

    assert result.ok
    assert len(result.warnings) == 1
    assert isinstance(result.warnings[0], Advisory)


def test_layers_compose_sorted() -> None:
    """Schema and semantic findings compose into a single sorted result."""
    descriptor = _descriptor(
        title="x" * 201,
        sections=[
            {
                "id": "root",
                "title": "Root",
                "description": "Root section.",
                "source_ids": ["s1"],
            }
        ],
    )

    result = validate(descriptor)

    assert result.codes == frozenset({"AKB002", "AKB005"})
    assert result.findings == tuple(sorted(result.findings))


def test_strict_adds_akb006() -> None:
    descriptor = _descriptor(next_minor_field=True)

    assert validate(descriptor).ok
    assert [finding.code for finding in validate(descriptor, strict=True).findings] == ["AKB006"]


def test_trailing_newline_id_rejected() -> None:
    """A trailing-newline id is AKB011 and never silently valid (schema+semantic agree)."""
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
    result = validate(descriptor)

    assert not result.ok
    assert "AKB011" in result.codes


def test_non_dict_input_invalid() -> None:
    result = validate("not a descriptor")

    assert not result.ok
    assert "AKB011" in result.codes


def test_public_types_are_exported() -> None:
    assert Finding.__name__ == "Finding"
    assert Advisory.__name__ == "Advisory"
    assert ValidationResult.__name__ == "ValidationResult"
