"""Schema-layer validation: bundled schemas plus normative keyword->code mapping.

Independent validators should emit identical codes for schema violations.
"""

from __future__ import annotations

import json
import re
from collections.abc import Callable, Iterator
from functools import cache
from importlib.resources import files
from typing import Any, cast

from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
from jsonschema.validators import extend

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


def _pattern_keyword(
    validator: Draft202012Validator, patrn: str, instance: object, schema: object
) -> Iterator[ValidationError]:
    """Validate `pattern` with ECMA-262 `$` semantics (JSON Schema's regex dialect).

    Python's `re` lets `$` match just before a trailing newline, so
    `^[a-z0-9_-]+$` wrongly accepts "s1\\n"; ECMA-262 (what the ajv gate uses)
    anchors `$` at end-of-input only. Anchoring `$` becomes `\\Z` so independent
    validators emit identical codes for identical documents (spec §7). This also
    governs `propertyNames`, whose subschema is validated through this keyword.
    """
    if isinstance(instance, str) and _compile_ecma(patrn).search(instance) is None:
        yield ValidationError(f"{instance!r} does not match {patrn!r}")


# The descriptor and provenance schemas share this ECMA-faithful `pattern` keyword.
# `jsonschema.validators.extend` is untyped in the stubs; cast it to a typed
# constructor so the derived validator carries the base validator's interface.
_extend = cast("Callable[..., type[Draft202012Validator]]", extend)
_EcmaValidator = _extend(Draft202012Validator, {"pattern": _pattern_keyword})


@cache
def _compile_ecma(patrn: str) -> re.Pattern[str]:
    return re.compile(_ecma_anchor(patrn))


def _ecma_anchor(pattern: str) -> str:
    r"""Rewrite anchoring `$` to `\Z` so `$` means end-of-input, as in ECMA-262.

    A `$` is a literal dollar when escaped or inside a character class; only
    unescaped, out-of-class anchors are rewritten. Character-class bounds honor
    the leading-`^` negation and a first-position literal `]`.
    """
    out: list[str] = []
    in_class = False
    escaped = False
    class_body = 0
    for char in pattern:
        if escaped:
            out.append(char)
            escaped = False
        elif char == "\\":
            out.append(char)
            escaped = True
        elif in_class:
            if char == "]" and class_body:
                in_class = False
            elif not (char == "^" and class_body == 0):
                class_body += 1
            out.append(char)
        elif char == "[":
            in_class = True
            class_body = 0
            out.append(char)
        elif char == "$":
            out.append(r"\Z")
        else:
            out.append(char)
    return "".join(out)


@cache
def descriptor_validator() -> Draft202012Validator:
    return _EcmaValidator(
        _load_schema("openakb.schema.json"),
        format_checker=_EcmaValidator.FORMAT_CHECKER,
    )


@cache
def provenance_validator() -> Draft202012Validator:
    return _EcmaValidator(
        _load_schema("provenance.schema.json"),
        format_checker=_EcmaValidator.FORMAT_CHECKER,
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
