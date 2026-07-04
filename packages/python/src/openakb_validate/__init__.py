"""openakb-validate: reference validator for the OpenAKB descriptor format (spec major v1)."""

from importlib.metadata import version as _distribution_version

from .citations import Citation, extract_citations
from .result import Advisory, Finding, ValidationResult
from .validator import validate

__version__ = _distribution_version("openakb-validate")

__all__ = [
    "Advisory",
    "Citation",
    "Finding",
    "ValidationResult",
    "__version__",
    "extract_citations",
    "validate",
]
