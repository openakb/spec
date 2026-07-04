"""Repository conformance fixtures exercised through the public package API."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from openakb_validate import extract_citations, validate

__all__ = ()

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CONFORMANCE = _REPO_ROOT / "conformance"

pytestmark = pytest.mark.skipif(
    not _CONFORMANCE.exists(),
    reason="repository conformance fixtures are absent",
)


def _case_dirs(kind: str) -> list[Path]:
    if not _CONFORMANCE.exists():
        return []
    return sorted(path for path in (_CONFORMANCE / kind).iterdir() if path.is_dir())


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _codes(descriptor: object, *, strict: bool = False) -> set[str]:
    return set(validate(descriptor, strict=strict).codes)


@pytest.mark.parametrize("case_dir", _case_dirs("valid"), ids=lambda path: path.name)
def test_valid_fixtures(case_dir: Path) -> None:
    """Valid descriptors pass in lenient and strict mode."""
    descriptor = _read_json(case_dir / "openakb.json")

    assert validate(descriptor).ok
    assert validate(descriptor, strict=True).ok


@pytest.mark.parametrize("case_dir", _case_dirs("invalid"), ids=lambda path: path.name)
def test_invalid_fixtures(case_dir: Path) -> None:
    """Invalid descriptors emit at least every code declared by the fixture."""
    descriptor = _read_json(case_dir / "openakb.json")
    expected = _read_json(case_dir / "expected.json")
    result = validate(descriptor)

    assert not result.ok
    assert set(expected["codes"]) <= set(result.codes)


@pytest.mark.parametrize("case_dir", _case_dirs("forward-compat"), ids=lambda path: path.name)
def test_forward_fixtures(case_dir: Path) -> None:
    """Forward-compatible descriptors are lenient-valid and strict-invalid as declared."""
    descriptor = _read_json(case_dir / "openakb.json")
    expected = _read_json(case_dir / "expected.json")

    assert validate(descriptor).ok
    assert set(expected["strict"]) <= _codes(descriptor, strict=True)


@pytest.mark.parametrize("case_dir", _case_dirs("content"), ids=lambda path: path.name)
def test_content_fixtures(case_dir: Path) -> None:
    """Citation parser output exactly matches fixture ids in document order."""
    markdown = (case_dir / "content.md").read_text(encoding="utf-8")
    expected = _read_json(case_dir / "expected.json")

    assert [list(citation.ids) for citation in extract_citations(markdown)] == [
        citation["ids"] for citation in expected["citations"]
    ]
