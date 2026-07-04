"""Cross-document semantic validation for descriptor-local references."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from typing import Any, Literal

from ._shape import indexed_dicts, is_local_id, reference_code
from .catalog import PARENT_DEPTH_MAX
from .result import Advisory, Finding, json_pointer

__all__ = ["semantic_findings", "semantic_warnings"]

_Kind = Literal["source", "section"]
_DEPTH_LIMIT = PARENT_DEPTH_MAX + 1


@dataclass(frozen=True)
class _IdEntry:
    kind: _Kind
    index: int
    value: str


def semantic_findings(descriptor: object) -> list[Finding]:
    """Find cross-document semantic violations not expressible in JSON Schema."""
    graph = _Graph.from_descriptor(descriptor)
    findings = [
        *_duplicate_ids(graph),
        *_empty_sections(graph),
        *_reference_findings(graph),
        *_tree_findings(graph),
    ]
    return sorted(findings)


def semantic_warnings(descriptor: object) -> list[Advisory]:
    """Emit MAY-warn advisories for discovery-graph cycles."""
    graph = _Graph.from_descriptor(descriptor)
    return _source_cycle_warnings(graph)


@dataclass(frozen=True)
class _Graph:
    sources: list[tuple[int, dict[str, Any]]]
    sections: list[tuple[int, dict[str, Any]]]
    source_ids: frozenset[str]
    section_ids: frozenset[str]

    @classmethod
    def from_descriptor(cls, descriptor: object) -> _Graph:
        if not isinstance(descriptor, dict):
            return cls(sources=[], sections=[], source_ids=frozenset(), section_ids=frozenset())
        sources = indexed_dicts(descriptor.get("sources"))
        sections = indexed_dicts(descriptor.get("sections"))
        return cls(
            sources=sources,
            sections=sections,
            source_ids=frozenset(_ids(sources)),
            section_ids=frozenset(_ids(sections)),
        )


def _duplicate_ids(graph: _Graph) -> list[Finding]:
    findings: list[Finding] = []
    seen: set[str] = set()
    for entry in _entries(graph.sources, "source"):
        if entry.value in seen:
            findings.append(_finding("AKB001", [f"{entry.kind}s", entry.index, "id"]))
        seen.add(entry.value)
    for entry in _entries(graph.sections, "section"):
        if entry.value in seen:
            findings.append(_finding("AKB001", [f"{entry.kind}s", entry.index, "id"]))
        seen.add(entry.value)
    return findings


def _empty_sections(graph: _Graph) -> list[Finding]:
    child_parent_ids = {
        section.get("parent_id")
        for _, section in graph.sections
        if is_local_id(section.get("parent_id"))
    }
    return [
        _finding("AKB002", ["sections", index])
        for index, section in graph.sections
        if not section.get("content_uri")
        and is_local_id(section.get("id"))
        and section["id"] not in child_parent_ids
    ]


def _reference_findings(graph: _Graph) -> list[Finding]:
    findings: list[Finding] = []
    for index, source in graph.sources:
        _append_ref(
            finding=findings,
            graph=graph,
            expected="source",
            value=source.get("discovered_via_id"),
            path=["sources", index, "discovered_via_id"],
        )
    for index, section in graph.sections:
        _append_ref(
            finding=findings,
            graph=graph,
            expected="section",
            value=section.get("parent_id"),
            path=["sections", index, "parent_id"],
        )
        _append_source_ids(
            findings, graph, section.get("source_ids"), ["sections", index, "source_ids"]
        )
        _append_link_findings(findings, graph, section, index)
        _append_claim_findings(findings, graph, section, index)
    return findings


def _tree_findings(graph: _Graph) -> list[Finding]:
    parent_by_id = _parent_by_id(graph)
    findings = _parent_cycle_findings(graph, parent_by_id)
    findings.extend(_depth_findings(graph, parent_by_id))
    return findings


def _source_cycle_warnings(graph: _Graph) -> list[Advisory]:
    next_by_id: dict[str, str] = {}
    for _, source in graph.sources:
        source_id = source.get("id")
        discovered_via_id = source.get("discovered_via_id")
        if (
            isinstance(source_id, str)
            and isinstance(discovered_via_id, str)
            and is_local_id(source_id)
            and is_local_id(discovered_via_id)
        ):
            next_by_id[source_id] = discovered_via_id
    index_by_id = _index_by_id(graph.sources)
    cycles = _cycles(next_by_id)
    warnings: list[Advisory] = []
    for cycle in sorted(cycles):
        path: list[str | int] = ["sources", index_by_id.get(cycle[0], 0), "discovered_via_id"]
        warnings.append(
            Advisory(
                path=json_pointer(path), message=f"discovered_via_id cycle: {' -> '.join(cycle)}"
            )
        )
    return warnings


def _append_ref(
    *,
    finding: list[Finding],
    graph: _Graph,
    expected: _Kind,
    value: object,
    path: list[str | int],
) -> None:
    if code := reference_code(value, expected, graph.source_ids, graph.section_ids):
        finding.append(_finding(code, path))


def _append_source_ids(
    findings: list[Finding],
    graph: _Graph,
    value: object,
    path: list[str | int],
) -> None:
    if not isinstance(value, list):
        return
    for index, item in enumerate(value):
        _append_ref(
            finding=findings,
            graph=graph,
            expected="source",
            value=item,
            path=[*path, index],
        )


def _append_link_findings(
    findings: list[Finding],
    graph: _Graph,
    section: dict[str, Any],
    section_index: int,
) -> None:
    for link_index, link in indexed_dicts(section.get("links")):
        if "akb_uri" in link:
            continue
        _append_ref(
            finding=findings,
            graph=graph,
            expected="section",
            value=link.get("section_id"),
            path=["sections", section_index, "links", link_index, "section_id"],
        )


def _append_claim_findings(
    findings: list[Finding],
    graph: _Graph,
    section: dict[str, Any],
    section_index: int,
) -> None:
    for claim_index, claim in indexed_dicts(section.get("provenance")):
        _append_source_ids(
            findings,
            graph,
            claim.get("source_ids"),
            ["sections", section_index, "provenance", claim_index, "source_ids"],
        )


def _parent_cycle_findings(
    graph: _Graph,
    parent_by_id: dict[str, str],
) -> list[Finding]:
    index_by_id = _index_by_id(graph.sections)
    return [
        _finding("AKB004", ["sections", index_by_id.get(cycle[0], 0), "parent_id"])
        for cycle in sorted(_cycles(parent_by_id))
    ]


def _depth_findings(
    graph: _Graph,
    parent_by_id: dict[str, str],
) -> list[Finding]:
    findings: list[Finding] = []
    for section_id, index in _index_by_id(graph.sections).items():
        depth = _section_depth(section_id, parent_by_id)
        if depth > PARENT_DEPTH_MAX:
            findings.append(_finding("AKB005", ["sections", index, "parent_id"]))
    return findings


def _section_depth(section_id: str, parent_by_id: dict[str, str]) -> int:
    depth = 1
    seen = {section_id}
    current = section_id
    while (parent_id := parent_by_id.get(current)) is not None:
        if parent_id in seen:
            return 1
        seen.add(parent_id)
        depth += 1
        current = parent_id
        if depth > _DEPTH_LIMIT:
            return depth
    return depth


def _cycles(next_by_id: dict[str, str]) -> set[tuple[str, ...]]:
    cycles: set[tuple[str, ...]] = set()
    for start in next_by_id:
        path: list[str] = []
        position: dict[str, int] = {}
        current = start
        while current in next_by_id:
            if current in position:
                cycles.add(_canonical_cycle(path[position[current] :]))
                break
            if current in path:
                break
            position[current] = len(path)
            path.append(current)
            current = next_by_id[current]
    return cycles


def _canonical_cycle(cycle: list[str]) -> tuple[str, ...]:
    if len(cycle) == 1:
        return (cycle[0],)
    rotations = [tuple(cycle[index:] + cycle[:index]) for index in range(len(cycle))]
    return min(rotations)


def _parent_by_id(graph: _Graph) -> dict[str, str]:
    return {
        section["id"]: section["parent_id"]
        for _, section in graph.sections
        if is_local_id(section.get("id"))
        and is_local_id(section.get("parent_id"))
        and section["parent_id"] in graph.section_ids
    }


def _index_by_id(items: Iterable[tuple[int, dict[str, Any]]]) -> dict[str, int]:
    return {item["id"]: index for index, item in items if is_local_id(item.get("id"))}


def _entries(items: Iterable[tuple[int, dict[str, Any]]], kind: _Kind) -> list[_IdEntry]:
    return [
        _IdEntry(kind=kind, index=index, value=item["id"])
        for index, item in items
        if is_local_id(item.get("id"))
    ]


def _ids(items: Iterable[tuple[int, dict[str, Any]]]) -> list[str]:
    return [item["id"] for _, item in items if is_local_id(item.get("id"))]


def _finding(code: str, path: list[str | int]) -> Finding:
    return Finding(code=code, path=json_pointer(path), message=code)
