"""Strict lint for unknown core members outside extension payloads.

Spec section 6's strict mode is exposed by this library as `validate(strict=True)`,
not as a CLI flag. A member outside the known core set and not under `x` is AKB006:
a typo or vendor data that belongs under `x`. Known-key sets are derived from the
bundled published schema, which is drift-gated against schema/v1/, so there is no
second hand-maintained field table. The lenient default never runs this.
"""

from __future__ import annotations

from functools import cache
from typing import Any, cast

from ._shape import indexed_dicts
from .result import Finding, json_pointer
from .schema import descriptor_validator

__all__ = ["strict_findings"]

_Parts = list[str | int]


def strict_findings(descriptor: object) -> list[Finding]:
    findings: list[Finding] = []
    if not isinstance(descriptor, dict):
        return findings
    _lint(descriptor, "top", [], findings)
    for index, source in indexed_dicts(descriptor.get("sources")):
        _lint(source, "source", ["sources", index], findings)
    for index, section in indexed_dicts(descriptor.get("sections")):
        parts: _Parts = ["sections", index]
        _lint(section, "section", parts, findings)
        for claim_index, claim in indexed_dicts(section.get("provenance")):
            claim_parts: _Parts = [*parts, "provenance", claim_index]
            _lint(claim, "claim", claim_parts, findings)
            locator = claim.get("locator")
            if isinstance(locator, dict):
                _lint(locator, "locator", [*claim_parts, "locator"], findings)
        for link_index, link in indexed_dicts(section.get("links")):
            _lint(link, "link", [*parts, "links", link_index], findings)
    return findings


@cache
def _known_keys() -> dict[str, frozenset[str]]:
    schema = cast("dict[str, Any]", descriptor_validator().schema)
    defs = schema["$defs"]
    return {
        "top": frozenset(schema["properties"]),
        "source": frozenset(defs["source"]["properties"]),
        "section": frozenset(defs["section"]["properties"]),
        "claim": frozenset(defs["claim"]["properties"]),
        "locator": frozenset(defs["claim"]["properties"]["locator"]["properties"]),
        "link": frozenset(defs["link"]["properties"]),
    }


def _lint(obj: dict[str, Any], kind: str, parts: _Parts, findings: list[Finding]) -> None:
    for key in obj:
        if key in _known_keys()[kind]:
            continue
        findings.append(
            Finding(
                code="AKB006",
                path=json_pointer([*parts, key]),
                message=f"unknown core member {key!r}: forward-minor data is tolerated"
                " by the lenient default; vendor data belongs under 'x'",
            )
        )
