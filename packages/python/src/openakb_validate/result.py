"""Findings and validation results."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass

from .catalog import CODE_NAMES

__all__ = ["Advisory", "Finding", "ValidationResult", "json_pointer"]


def json_pointer(parts: Iterable[str | int]) -> str:
    """RFC 6901 JSON Pointer from path segments; the root is the empty string."""
    return "".join("/" + str(part).replace("~", "~0").replace("/", "~1") for part in parts)


@dataclass(frozen=True, order=True)
class Finding:
    """One violation of a normative rule, carrying its stable error code."""

    code: str
    path: str
    message: str

    @property
    def name(self) -> str:
        """Public finding name for this code; unknown codes echo the code itself.

        `Finding` is user-constructible, so an out-of-catalog code returns the code
        rather than raising, keeping the accessor total for callers.
        """
        return CODE_NAMES.get(self.code, self.code)


@dataclass(frozen=True)
class Advisory:
    """A code-less warning for the spec's MAY-warn surfaces; never affects a verdict."""

    path: str
    message: str


@dataclass(frozen=True)
class ValidationResult:
    """The outcome of structural validation: valid iff there are no findings.

    Warnings are advisory (spec MAY-warn) and never affect `ok`.
    """

    findings: tuple[Finding, ...]
    warnings: tuple[Advisory, ...] = ()

    @property
    def ok(self) -> bool:
        return not self.findings

    @property
    def codes(self) -> frozenset[str]:
        return frozenset(finding.code for finding in self.findings)
