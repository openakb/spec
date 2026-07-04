"""Repository examples validate through the public library API."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from openakb_validate import (
    UNVERIFIABLE,
    VERIFIED,
    LocalFileResolver,
    check_content,
    validate,
)

__all__ = ()

_REPO_ROOT = Path(__file__).resolve().parents[3]
_EXAMPLES = _REPO_ROOT / "examples"

pytestmark = pytest.mark.skipif(not _EXAMPLES.exists(), reason="examples tree absent")

_AUTHORING = ["minimal", "widget-platform", "cross-link", "sidecar-provenance"]


def _load(example: str) -> dict[str, Any]:
    descriptor = json.loads((_EXAMPLES / example / "openakb.json").read_text(encoding="utf-8"))
    assert isinstance(descriptor, dict)
    return descriptor


@pytest.mark.parametrize("example", [*_AUTHORING, "widget-platform-served"])
def test_example_validates(example: str) -> None:
    descriptor = _load(example)

    result = validate(descriptor, strict=True)

    assert result.ok


@pytest.mark.parametrize("example", _AUTHORING)
def test_authoring_content(example: str) -> None:
    descriptor = _load(example)
    directory = _EXAMPLES / example

    report = check_content(descriptor, LocalFileResolver(base_dir=directory))

    assert report.ok
    assert report.checks
    assert not report.failed
    for check in report.checks:
        if check.kind == "quote":
            assert check.outcome in {VERIFIED, UNVERIFIABLE}
            if check.outcome == UNVERIFIABLE:
                assert check.detail == "no cited source capture fetched"
        else:
            assert check.outcome == VERIFIED


def test_widget_artifacts() -> None:
    descriptor = _load("widget-platform")
    directory = _EXAMPLES / "widget-platform"

    report = check_content(descriptor, LocalFileResolver(base_dir=directory))

    verified = {check.kind for check in report.checks if check.outcome == VERIFIED}
    assert {"capture", "sidecar", "citations"} <= verified
    assert not report.failed


def test_served_unverifiable() -> None:
    descriptor = _load("widget-platform-served")
    directory = _EXAMPLES / "widget-platform-served"

    report = check_content(descriptor, LocalFileResolver(base_dir=directory))

    assert report.ok
    assert report.checks
    assert {check.outcome for check in report.checks} == {UNVERIFIABLE}
