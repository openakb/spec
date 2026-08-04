"""Cross-document semantic rules: AKB001/002/004/005/007/010 + MAY-warn advisories."""

import time
import tracemalloc
from typing import Any

from openakb_validate import validate
from openakb_validate.catalog import PARENT_DEPTH_MAX
from openakb_validate.result import Finding
from openakb_validate.semantic import semantic_findings, semantic_warnings

__all__ = ()

# A parent chain long enough that the pre-fix O(n^3) cycle scan would take minutes; the
# linear rework must clear it well within the ceiling on ordinary hardware.
_LARGE_CHAIN_SECTIONS = 3000
_LARGE_CHAIN_CEILING_SECONDS = 2.0

# A closed parent_id ring large enough that the pre-fix O(n^2) rotation-materializing
# canonicalization would burn ~800 MB / ~4s; the O(n) rewrite must clear it cheaply.
_RING_SECTIONS = 10_000

# The O(n) canonicalization peaks at ~4.5 MB for the whole validate() call on a
# 10k-node ring; a reintroduced O(n^2) rotation-materializing implementation peaks at
# ~800 MB for the same input. 100 MB is a wide margin over the O(n) profile and far
# below the O(n^2) one, so it hard-fails a quadratic regression without being flaky.
_RING_MEMORY_CEILING_BYTES = 100 * 1024 * 1024


def _descriptor(**overrides: object) -> dict[str, Any]:
    base: dict[str, Any] = {
        "$schema": "https://schema.openakb.org/v1/openakb.schema.json",
        "id": "kb",
        "title": "KB",
        "description": "A test descriptor.",
        "sources": [{"id": "SRC-000001", "type": "url", "uri": "https://docs.example.com/"}],
        "sections": [
            {
                "id": "SEC-000001",
                "title": "Root",
                "description": "Root section.",
                "content_uri": "root.md",
                "source_ids": ["SRC-000001"],
            }
        ],
    }
    base.update(overrides)
    return base


def _codes(descriptor: object) -> set[str]:
    return {finding.code for finding in semantic_findings(descriptor)}


def _section(section_id: str, **extra: object) -> dict[str, Any]:
    section: dict[str, Any] = {
        "id": section_id,
        "title": section_id.title(),
        "description": f"Section {section_id}.",
        "content_uri": f"{section_id}.md",
        "source_ids": ["SRC-000001"],
    }
    section.update(extra)
    return section


def test_clean_descriptor() -> None:
    """A minimal well-formed descriptor produces no semantic findings."""
    assert semantic_findings(_descriptor()) == []


def test_akb001_shared_space() -> None:
    """Section IDs share the source ID namespace."""
    descriptor = _descriptor(sections=[_section("SRC-000001")])
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [("AKB001", "/sections/0/id")]


def test_akb001_duplicate_sections() -> None:
    """Two sections sharing an id report a single AKB001 at the second section."""
    descriptor = _descriptor(sections=[_section("SEC-000001"), _section("SEC-000001")])
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [("AKB001", "/sections/1/id")]


def test_case_insensitive_duplicate_ids_akb001() -> None:
    """Ids differing only in ASCII case are duplicates under the casefolded policy."""
    descriptor = _descriptor(sections=[_section("SEC-00000A"), _section("SEC-00000a")])
    assert "AKB001" in _codes(descriptor)


def test_akb002_empty_section() -> None:
    """A section without content_uri at all is reported as AKB002."""
    section = _section("SEC-000001")
    del section["content_uri"]
    descriptor = _descriptor(sections=[section])
    assert "AKB002" in _codes(descriptor)


def test_empty_content_uri() -> None:
    """A present empty URI reference is semantic content; schema owns its shape."""
    descriptor = _descriptor(sections=[_section("SEC-000001", content_uri="")])
    assert "AKB002" not in _codes(descriptor)


def test_null_content_uri() -> None:
    """A present malformed content_uri must not cascade into AKB002."""
    descriptor = _descriptor(sections=[_section("SEC-000001", content_uri=None)])
    assert "AKB002" not in _codes(descriptor)


def test_container_with_child() -> None:
    """A container section (null content_uri) with a resolvable child has no findings."""
    sections = [
        _section("SEC-000001", content_uri=None),
        _section("SEC-000002", parent_id="SEC-000001"),
    ]
    assert semantic_findings(_descriptor(sections=sections)) == []


def test_akb004_parent_cycle() -> None:
    """A two-section parent cycle emits exactly one AKB004 finding."""
    sections = [
        _section("SEC-000001", parent_id="SEC-000002"),
        _section("SEC-000002", parent_id="SEC-000001"),
    ]
    findings = semantic_findings(_descriptor(sections=sections))
    assert [finding.code for finding in findings].count("AKB004") == 1


def test_akb004_self_parent() -> None:
    """A section whose parent_id points to itself is reported as AKB004."""
    descriptor = _descriptor(sections=[_section("SEC-000001", parent_id="SEC-000001")])
    assert "AKB004" in _codes(descriptor)


def test_akb005_depth_over() -> None:
    """Root depth is one, so SEC-000064 in a chain SEC-000000..SEC-000064 exceeds the cap."""
    sections = [
        _section(
            f"SEC-{index:06d}",
            **({"parent_id": f"SEC-{index - 1:06d}"} if index else {}),
        )
        for index in range(65)
    ]
    findings = semantic_findings(_descriptor(sections=sections))
    assert [finding.code for finding in findings].count("AKB005") == 1


def test_depth_64_allowed() -> None:
    """A parent chain of exactly the depth cap (64) does not trigger AKB005."""
    sections = [
        _section(
            f"SEC-{index:06d}",
            **({"parent_id": f"SEC-{index - 1:06d}"} if index else {}),
        )
        for index in range(64)
    ]
    assert "AKB005" not in _codes(_descriptor(sections=sections))


def test_akb007_unresolved_parent() -> None:
    """A parent_id referencing a nonexistent id is reported as AKB007."""
    descriptor = _descriptor(sections=[_section("SEC-000001", parent_id="SEC-999999")])
    assert "AKB007" in _codes(descriptor)


def test_akb010_parent_source() -> None:
    """A parent_id pointing at a source id (wrong kind) is reported as AKB010."""
    descriptor = _descriptor(sections=[_section("SEC-000001", parent_id="SRC-000001")])
    assert "AKB010" in _codes(descriptor)


def test_akb007_unresolved_source() -> None:
    """A section's source_ids referencing a nonexistent id is reported as AKB007."""
    descriptor = _descriptor(sections=[_section("SEC-000001", source_ids=["SRC-999999"])])
    assert "AKB007" in _codes(descriptor)


def test_akb010_source_section() -> None:
    """A section's source_ids pointing at a section id (wrong kind) is reported as AKB010."""
    descriptor = _descriptor(sections=[_section("SEC-000001", source_ids=["SEC-000001"])])
    assert "AKB010" in _codes(descriptor)


def test_akb007_unresolved_discovery() -> None:
    """A source's discovered_via_id referencing a nonexistent id is reported as AKB007."""
    descriptor = _descriptor(sources=[{"id": "SRC-000001", "discovered_via_id": "SRC-999999"}])
    assert "AKB007" in _codes(descriptor)


def test_akb010_discovery_section() -> None:
    """A source's discovered_via_id pointing at a section id (wrong kind) is AKB010."""
    descriptor = _descriptor(sources=[{"id": "SRC-000001", "discovered_via_id": "SEC-000001"}])
    assert "AKB010" in _codes(descriptor)


def test_local_cross_links() -> None:
    """Local links validate section_id; cross-AKB links with akb_uri do not."""
    descriptor = _descriptor(
        sections=[
            _section(
                "SEC-000001",
                links=[
                    {"rel": "see-also", "section_id": "SEC-999999"},
                    {
                        "rel": "see-also",
                        "akb_uri": "https://kb.example.com/openakb.json",
                        "section_id": "SEC-999999",
                    },
                ],
            )
        ]
    )
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [
        ("AKB007", "/sections/0/links/0/section_id")
    ]


def test_akb010_link_source() -> None:
    """A same-AKB section link cannot point at a source ID."""
    descriptor = _descriptor(
        sections=[_section("SEC-000001", links=[{"rel": "see-also", "section_id": "SRC-000001"}])]
    )
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [
        ("AKB010", "/sections/0/links/0/section_id")
    ]


def test_inline_claim_sources() -> None:
    """An inline claim's source_ids referencing a nonexistent id is reported as AKB007."""
    descriptor = _descriptor(
        sections=[_section("SEC-000001", provenance=[{"source_ids": ["SRC-999999"]}])]
    )
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [
        ("AKB007", "/sections/0/provenance/0/source_ids/0")
    ]


def test_akb010_claim_section() -> None:
    """Inline claim source IDs cannot point at section IDs."""
    descriptor = _descriptor(
        sections=[_section("SEC-000001", provenance=[{"source_ids": ["SEC-000001"]}])]
    )
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [
        ("AKB010", "/sections/0/provenance/0/source_ids/0")
    ]


def test_invalid_tokens_skipped() -> None:
    """A parent_id failing the id pattern is left to the schema layer, not reported here."""
    descriptor = _descriptor(sections=[_section("SEC-000001", parent_id="Bad Id")])
    assert semantic_findings(descriptor) == []


def test_arbitrary_shapes() -> None:
    """Arbitrary malformed or None input never raises and yields no findings or warnings."""
    malformed = {"sources": [None, {"id": []}], "sections": [42, {"id": {}}]}
    assert semantic_findings(None) == []
    assert semantic_findings(malformed) == []
    assert semantic_warnings(None) == []


def test_discovery_cycle_advisory() -> None:
    """A two-source discovered_via_id cycle emits a single advisory mentioning 'cycle'."""
    sources = [
        {"id": "SRC-000001", "discovered_via_id": "SRC-000002"},
        {"id": "SRC-000002", "discovered_via_id": "SRC-000001"},
    ]
    warnings = semantic_warnings(_descriptor(sources=sources))
    assert len(warnings) == 1
    assert "cycle" in warnings[0].message


def test_acyclic_discovery() -> None:
    """A non-cyclic discovered_via_id chain emits no advisories."""
    sources = [
        {"id": "SRC-000001"},
        {"id": "SRC-000002", "discovered_via_id": "SRC-000001"},
    ]
    assert semantic_warnings(_descriptor(sources=sources)) == []


def _sole_finding(descriptor: object, code: str) -> Finding:
    matches = [finding for finding in semantic_findings(descriptor) if finding.code == code]
    assert len(matches) == 1, f"expected exactly one {code}, got {matches}"
    return matches[0]


def test_akb001_message_names_duplicate() -> None:
    """The duplicate-id message quotes the id and its first-declared location."""
    descriptor = _descriptor(sections=[_section("SEC-000001"), _section("SEC-000001")])
    finding = _sole_finding(descriptor, "AKB001")
    assert '"SEC-000001"' in finding.message
    assert "/sections/0/id" in finding.message


def test_akb001_duplicate_source_ids() -> None:
    """Two sources sharing an id emit a single AKB001 at the second source."""
    sources = [
        {"id": "SRC-000001", "type": "url", "uri": "https://docs.example.com/a/"},
        {"id": "SRC-000001", "type": "url", "uri": "https://docs.example.com/b/"},
    ]
    descriptor = _descriptor(
        sources=sources, sections=[_section("SEC-000001", source_ids=["SRC-000001"])]
    )
    findings = semantic_findings(descriptor)
    assert [(finding.code, finding.path) for finding in findings] == [("AKB001", "/sources/1/id")]


def test_akb002_message_names_section() -> None:
    """The empty-section message quotes the offending section id."""
    section = _section("SEC-00000F")
    del section["content_uri"]
    finding = _sole_finding(_descriptor(sections=[section]), "AKB002")
    assert '"SEC-00000F"' in finding.message


def test_akb004_message_renders_cycle() -> None:
    """The parent-cycle message renders the closed chain a -> b -> a."""
    sections = [
        _section("SEC-000001", parent_id="SEC-000002"),
        _section("SEC-000002", parent_id="SEC-000001"),
    ]
    finding = _sole_finding(_descriptor(sections=sections), "AKB004")
    assert "sec-000001 -> sec-000002 -> sec-000001" in finding.message


def test_akb005_message_states_depth() -> None:
    """The cap message states the offending depth and the maximum."""
    sections = [
        _section(
            f"SEC-{index:06d}",
            **({"parent_id": f"SEC-{index - 1:06d}"} if index else {}),
        )
        for index in range(PARENT_DEPTH_MAX + 1)
    ]
    finding = _sole_finding(_descriptor(sections=sections), "AKB005")
    assert str(PARENT_DEPTH_MAX + 1) in finding.message
    assert str(PARENT_DEPTH_MAX) in finding.message


def test_akb007_message_names_reference() -> None:
    """The unresolved-reference message quotes the offending id."""
    descriptor = _descriptor(sections=[_section("SEC-000001", parent_id="SEC-999999")])
    finding = _sole_finding(descriptor, "AKB007")
    assert '"SEC-999999"' in finding.message


def test_akb010_message_states_kind() -> None:
    """The wrong-kind message quotes the id and names the expected kind."""
    descriptor = _descriptor(sections=[_section("SEC-000001", parent_id="SRC-000001")])
    finding = _sole_finding(descriptor, "AKB010")
    assert '"SRC-000001"' in finding.message
    assert "section" in finding.message


def test_advisory_message_renders_cycle() -> None:
    """The discovery-cycle advisory renders the closed chain a -> b -> a."""
    sources = [
        {"id": "SRC-000001", "discovered_via_id": "SRC-000002"},
        {"id": "SRC-000002", "discovered_via_id": "SRC-000001"},
    ]
    warnings = semantic_warnings(_descriptor(sources=sources))
    assert "src-000001 -> src-000002 -> src-000001" in warnings[0].message


def test_parent_cycle_three_nodes() -> None:
    """A three-section parent cycle collapses to one AKB004."""
    sections = [
        _section("SEC-000001", parent_id="SEC-000003"),
        _section("SEC-000002", parent_id="SEC-000001"),
        _section("SEC-000003", parent_id="SEC-000002"),
    ]
    findings = semantic_findings(_descriptor(sections=sections))
    assert [finding.code for finding in findings].count("AKB004") == 1


def test_chain_into_cycle_once() -> None:
    """A tail feeding a cycle reports that cycle a single time."""
    sections = [
        _section("SEC-000003", parent_id="SEC-000001"),
        _section("SEC-000001", parent_id="SEC-000002"),
        _section("SEC-000002", parent_id="SEC-000001"),
    ]
    findings = semantic_findings(_descriptor(sections=sections))
    assert [finding.code for finding in findings].count("AKB004") == 1


def test_discovery_cycle_three_nodes() -> None:
    """A three-source discovery cycle yields exactly one advisory."""
    sources = [
        {"id": "SRC-000001", "discovered_via_id": "SRC-000003"},
        {"id": "SRC-000002", "discovered_via_id": "SRC-000001"},
        {"id": "SRC-000003", "discovered_via_id": "SRC-000002"},
    ]
    assert len(semantic_warnings(_descriptor(sources=sources))) == 1


def test_validate_large_chain_fast() -> None:
    """A long linear parent chain validates well within the perf ceiling (S11)."""
    sections = [
        _section(
            f"SEC-{index:06d}",
            **({"parent_id": f"SEC-{index - 1:06d}"} if index else {}),
        )
        for index in range(_LARGE_CHAIN_SECTIONS)
    ]
    descriptor = _descriptor(sections=sections)
    start = time.perf_counter()
    validate(descriptor)
    elapsed = time.perf_counter() - start
    assert elapsed < _LARGE_CHAIN_CEILING_SECONDS, f"validate() took {elapsed:.2f}s"


def test_large_ring_cycle_completes() -> None:
    """A 10k-node parent_id ring reports one canonical AKB004 cycle within a memory ceiling."""
    ids = [f"SEC-{index:06d}" for index in range(_RING_SECTIONS)]
    sections = [
        _section(section_id, parent_id=ids[(index + 1) % _RING_SECTIONS])
        for index, section_id in enumerate(ids)
    ]
    descriptor = _descriptor(sections=sections)
    tracemalloc.start()
    try:
        result = validate(descriptor)
        _, peak_bytes = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()
    assert peak_bytes < _RING_MEMORY_CEILING_BYTES, f"validate() peaked at {peak_bytes} bytes"
    cycle_findings = [finding for finding in result.findings if finding.code == "AKB004"]
    assert len(cycle_findings) == 1
    # Cycle rendering uses normalized (lowercased) ids.
    normalized_ids = [identifier.lower() for identifier in ids]
    expected_message = f"parent_id cycle: {' -> '.join([*normalized_ids, normalized_ids[0]])}"
    assert cycle_findings[0].message == expected_message
