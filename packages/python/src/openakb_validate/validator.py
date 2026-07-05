"""The public validate() facade (spec §7)."""

from __future__ import annotations

from dataclasses import dataclass

from .content import ContentReport, Resolver, check_content
from .result import ValidationResult
from .schema import schema_findings
from .semantic import semantic_findings, semantic_warnings
from .strict import strict_findings

__all__ = ["FullReport", "validate", "validate_with_content"]


@dataclass(frozen=True)
class FullReport:
    """Combined structural and opt-in content validation result."""

    validation: ValidationResult
    content: ContentReport

    @property
    def ok(self) -> bool:
        return self.validation.ok and self.content.ok


def validate(descriptor: object, *, strict: bool = False) -> ValidationResult:
    """Structurally validate a parsed OpenAKB descriptor.

    The lenient default tolerates unknown core members for forward compatibility;
    strict=True adds the AKB006 unknown-core-property lint (spec §6). Content behind
    URIs is never fetched here -- see check_content() for the opt-in checks. Warnings
    are advisory (spec MAY-warn) and never affect the verdict.
    """
    findings = schema_findings(descriptor) + semantic_findings(descriptor)
    if strict:
        findings += strict_findings(descriptor)
    return ValidationResult(
        findings=tuple(sorted(findings)),
        warnings=tuple(semantic_warnings(descriptor)),
    )


def validate_with_content(
    descriptor: object, resolver: Resolver, *, strict: bool = False
) -> FullReport:
    """Run structural validation and opt-in content checks."""
    return FullReport(
        validation=validate(descriptor, strict=strict),
        content=check_content(descriptor, resolver),
    )
