"""AKB006 strict lint: unknown core members outside `x` (spec §6)."""

from typing import Any

from openakb_validate.strict import strict_findings

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


def test_clean_descriptor() -> None:
    assert strict_findings(_descriptor()) == []


def test_unknown_top_level() -> None:
    findings = strict_findings(_descriptor(sponsor="example"))
    assert [(finding.code, finding.path) for finding in findings] == [("AKB006", "/sponsor")]


def test_x_not_descended() -> None:
    """Extension payloads are tolerated and their contents are never linted."""
    descriptor = _descriptor(x={"com.example": {"anything": {"nested": True}}})
    assert strict_findings(descriptor) == []


def test_unknown_nested_members() -> None:
    """Unknown members are reported at every strict-linted schema level."""
    descriptor = _descriptor(
        sources=[
            {
                "id": "s1",
                "type": "url",
                "uri": "https://docs.example.com/",
                "oops_source": True,
            }
        ],
        sections=[
            {
                "id": "root",
                "title": "Root",
                "description": "Root section.",
                "content_uri": "root.md",
                "source_ids": ["s1"],
                "oops_section": True,
                "provenance": [
                    {
                        "source_ids": ["s1"],
                        "locator": {"quote": "a", "oops_locator": True},
                        "oops_claim": True,
                    }
                ],
                "links": [
                    {
                        "rel": "see-also",
                        "section_id": "root",
                        "oops_link": True,
                    }
                ],
            }
        ],
    )

    assert [(finding.code, finding.path) for finding in strict_findings(descriptor)] == [
        ("AKB006", "/sources/0/oops_source"),
        ("AKB006", "/sections/0/oops_section"),
        ("AKB006", "/sections/0/provenance/0/oops_claim"),
        ("AKB006", "/sections/0/provenance/0/locator/oops_locator"),
        ("AKB006", "/sections/0/links/0/oops_link"),
    ]


def test_non_dict_input() -> None:
    assert strict_findings(None) == []
