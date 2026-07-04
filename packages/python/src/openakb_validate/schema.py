"""Schema-layer validation: bundled schemas plus normative keyword->code mapping.

Independent validators should emit identical codes for schema violations.
"""

from __future__ import annotations

import json
from functools import cache
from importlib.resources import files
from typing import Any

from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError

from .result import Finding, json_pointer

__all__ = ["descriptor_validator", "provenance_validator", "schema_findings"]

_KEYWORD_CODES: dict[str, str] = {
    "maxLength": "AKB005",
    "maxItems": "AKB005",
    "required": "AKB009",
    "pattern": "AKB011",
    "format": "AKB011",
    "type": "AKB011",
    "minimum": "AKB011",
    "minLength": "AKB011",
    "minItems": "AKB011",
    "uniqueItems": "AKB011",
    "enum": "AKB011",
    "anyOf": "AKB011",
    "propertyNames": "AKB011",
}


@cache
def descriptor_validator() -> Draft202012Validator:
    return Draft202012Validator(
        _load_schema("openakb.schema.json"),
        format_checker=Draft202012Validator.FORMAT_CHECKER,
    )


@cache
def provenance_validator() -> Draft202012Validator:
    return Draft202012Validator(
        _load_schema("provenance.schema.json"),
        format_checker=Draft202012Validator.FORMAT_CHECKER,
    )


def schema_findings(
    instance: object, validator: Draft202012Validator | None = None
) -> list[Finding]:
    """Validate against a bundled schema (default: the descriptor schema)."""
    checker = validator if validator is not None else descriptor_validator()
    findings = [
        Finding(code=code, path=json_pointer(error.absolute_path), message=error.message)
        for error in checker.iter_errors(instance)
        if (code := _code_for(error)) is not None
    ]
    return sorted(findings)


def _load_schema(name: str) -> dict[str, Any]:
    text = files("openakb_validate.schemas").joinpath(name).read_text("utf-8")
    schema: dict[str, Any] = json.loads(text)
    return schema


def _code_for(error: ValidationError) -> str | None:
    """Map one keyword violation to its stable code (spec §7)."""
    path = list(error.absolute_path)
    if path and path[-1] == "rel":
        return "AKB011" if error.validator == "type" else "AKB008"
    if (
        error.validator == "anyOf"
        and len(path) >= 2
        and path[-2] == "links"
        and isinstance(path[-1], int)
    ):
        return "AKB012"
    if "then" in error.absolute_schema_path:
        return "AKB003"
    return _KEYWORD_CODES.get(str(error.validator))
