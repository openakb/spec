"""Finding/ValidationResult model and JSON Pointer building."""

from openakb_validate.result import Advisory, Finding, ValidationResult, json_pointer

__all__ = ()


def test_json_pointer_builds_and_escapes() -> None:
    """json_pointer joins path segments and escapes ~ and / per RFC 6901."""
    assert json_pointer([]) == ""
    assert json_pointer(["sections", 0, "id"]) == "/sections/0/id"
    assert json_pointer(["a/b", "c~d"]) == "/a~1b/c~0d"


def test_finding_name_derives_from_catalog() -> None:
    """Finding.name looks up the catalog's slug for a known error code."""
    finding = Finding(code="AKB004", path="/sections/0/parent_id", message="cycle")
    assert finding.name == "parent-cycle"


def test_finding_name_unknown_code() -> None:
    """An out-of-catalog code echoes the code itself back, never a bare KeyError."""
    finding = Finding(code="AKB999", path="/x", message="m")
    assert finding.name == "AKB999"


def test_findings_sort_order() -> None:
    """Findings sort by code first (dataclass field order), not by path."""
    a = Finding(code="AKB001", path="/z", message="m")
    b = Finding(code="AKB002", path="/a", message="m")
    assert sorted([b, a]) == [a, b]


def test_result_ok_and_codes() -> None:
    """ValidationResult.ok and .codes reflect an empty vs. non-empty findings tuple."""
    empty = ValidationResult(findings=())
    assert empty.ok and empty.codes == frozenset()
    result = ValidationResult(findings=(Finding(code="AKB009", path="", message="missing"),))
    assert not result.ok
    assert result.codes == frozenset({"AKB009"})


def test_advisory_is_a_value_object() -> None:
    """Two Advisory instances with equal fields compare equal."""
    advisory = Advisory(path="/sources/0/discovered_via_id", message="cycle")
    assert advisory == Advisory(path="/sources/0/discovered_via_id", message="cycle")


def test_warnings_never_affect_ok() -> None:
    """A ValidationResult with only warnings still reports ok."""
    result = ValidationResult(findings=(), warnings=(Advisory(path="", message="w"),))
    assert result.ok
