"""openakb-validate: reference validator for the OpenAKB descriptor format (spec major v1)."""

from importlib.metadata import version as _distribution_version

from .citations import Citation, extract_citations
from .content import (
    FAILED,
    UNVERIFIABLE,
    VERIFIED,
    ContentCheck,
    ContentReport,
    LocalFileResolver,
    Resolver,
    Unfetchable,
    check_content,
)
from .result import Advisory, Finding, ValidationResult
from .validator import FullReport, validate, validate_with_content

__version__ = _distribution_version("openakb-validate")

__all__ = [
    "FAILED",
    "UNVERIFIABLE",
    "VERIFIED",
    "Advisory",
    "Citation",
    "ContentCheck",
    "ContentReport",
    "Finding",
    "FullReport",
    "LocalFileResolver",
    "Resolver",
    "Unfetchable",
    "ValidationResult",
    "__version__",
    "check_content",
    "extract_citations",
    "validate",
    "validate_with_content",
]
