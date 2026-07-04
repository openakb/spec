"""Finding/ValidationResult model and JSON Pointer building."""

from openakb_validate.result import Advisory, Finding, ValidationResult, json_pointer


def test_json_pointer_builds_and_escapes() -> None:
    assert json_pointer([]) == ""
    assert json_pointer(["sections", 0, "id"]) == "/sections/0/id"
    assert json_pointer(["a/b", "c~d"]) == "/a~1b/c~0d"


def test_finding_name_derives_from_catalog() -> None:
    finding = Finding(code="AKB004", path="/sections/0/parent_id", message="cycle")
    assert finding.name == "parent-cycle"


def test_findings_sort_order() -> None:
    a = Finding(code="AKB001", path="/z", message="m")
    b = Finding(code="AKB002", path="/a", message="m")
    assert sorted([b, a]) == [a, b]


def test_result_ok_and_codes() -> None:
    empty = ValidationResult(findings=())
    assert empty.ok and empty.codes == frozenset()
    result = ValidationResult(findings=(Finding(code="AKB009", path="", message="missing"),))
    assert not result.ok
    assert result.codes == frozenset({"AKB009"})


def test_advisory_is_a_value_object() -> None:
    advisory = Advisory(path="/sources/0/discovered_via_id", message="cycle")
    assert advisory == Advisory(path="/sources/0/discovered_via_id", message="cycle")


def test_warnings_never_affect_ok() -> None:
    result = ValidationResult(findings=(), warnings=(Advisory(path="", message="w"),))
    assert result.ok
