"""Emit this package's cross-validator conformance report."""

from __future__ import annotations

import json
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import Any

from openakb_validate import __version__, extract_citations, validate

__all__ = ["main"]

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CONFORMANCE = _REPO_ROOT / "conformance"


def _case_dirs(kind: str) -> list[Path]:
    return sorted(path for path in (_CONFORMANCE / kind).iterdir() if path.is_dir())


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _codes(descriptor: object, *, strict: bool = False) -> list[str]:
    return sorted(validate(descriptor, strict=strict).codes)


def _cases() -> Iterator[tuple[str, dict[str, object]]]:
    for case_dir in _case_dirs("valid"):
        descriptor = _read_json(case_dir / "openakb.json")
        yield (
            f"valid/{case_dir.name}",
            {
                "lenient": _codes(descriptor),
                "strict": _codes(descriptor, strict=True),
            },
        )

    for case_dir in _case_dirs("invalid"):
        descriptor = _read_json(case_dir / "openakb.json")
        yield (
            f"invalid/{case_dir.name}",
            {
                "lenient": _codes(descriptor),
                "strict": _codes(descriptor, strict=True),
            },
        )

    for case_dir in _case_dirs("forward-compat"):
        descriptor = _read_json(case_dir / "openakb.json")
        yield (
            f"forward-compat/{case_dir.name}",
            {
                "lenient": _codes(descriptor),
                "strict": _codes(descriptor, strict=True),
            },
        )

    for case_dir in _case_dirs("content"):
        markdown = (case_dir / "content.md").read_text(encoding="utf-8")
        yield (
            f"content/{case_dir.name}",
            {"citations": [list(citation.ids) for citation in extract_citations(markdown)]},
        )


def main() -> int:
    report = {
        "implementation": "openakb-validate-python",
        "version": __version__,
        "spec_major": 1,
        "fixtures": dict(_cases()),
    }
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
