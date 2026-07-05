"""openakb-validate: reference validator for the OpenAKB descriptor format (spec major v1)."""

from importlib.metadata import version as _distribution_version

from .citations import Citation, extract_citations
from .content import (
    FAILED,
    KIND_CAPTURE,
    KIND_CITATIONS,
    KIND_CONTENT_HASH,
    KIND_GUIDE_HASH,
    KIND_QUOTE,
    KIND_SIDECAR,
    UNVERIFIABLE,
    VERIFIED,
    ContentCheck,
    ContentReport,
    LocalFileResolver,
    Resolver,
    Unfetchable,
    check_content,
)
from .result import Advisory, Finding, ValidationResult, json_pointer
from .validator import FullReport, validate, validate_with_content

__version__ = _distribution_version("openakb-validate")

__all__ = [
    "FAILED",
    "KIND_CAPTURE",
    "KIND_CITATIONS",
    "KIND_CONTENT_HASH",
    "KIND_GUIDE_HASH",
    "KIND_QUOTE",
    "KIND_SIDECAR",
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
    "json_pointer",
    "validate",
    "validate_with_content",
]
