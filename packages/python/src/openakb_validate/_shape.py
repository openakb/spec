"""Internal shape helpers shared by the semantic, strict, and content layers.

Everything here is defensive: descriptors arrive as arbitrary parsed JSON, and type
errors are the schema layer's job, so unexpected shapes are skipped, never raised on.
"""

from __future__ import annotations

import re
import string
from typing import Any, cast

__all__ = ["id_kind", "indexed_dicts", "is_typed_id", "normalize_id", "reference_code"]

# Explicit ASCII classes instead of re.IGNORECASE: Python's Unicode case folding would
# otherwise admit foldable non-ASCII characters (e.g. U+212A KELVIN SIGN, U+017F LONG S)
# that the schema's ASCII-only `[0-9A-Za-z]{6}` pattern rejects as AKB011.
_SECTION_ID_RE = re.compile(r"[Ss][Ee][Cc]-[0-9A-Za-z]{6}")
_SOURCE_ID_RE = re.compile(r"[Ss][Rr][Cc]-[0-9A-Za-z]{6}")

# `normalize_id` translates through this instead of `str.lower()` to mirror Rust's
# `to_ascii_lowercase`: an ASCII-only fold that leaves every non-ASCII byte
# untouched. `str.lower()` Unicode-folds confusables (e.g. U+212A KELVIN SIGN ->
# ASCII 'k') onto real ASCII keys, which the Rust validator never does -- that gap
# is what let a malformed id collide with a real one and diverge between the two
# validators. Valid typed ids are pure ASCII, so this table changes nothing for them.
_ASCII_LOWER = str.maketrans(string.ascii_uppercase, string.ascii_lowercase)


def is_typed_id(value: object) -> bool:
    """True iff value is a string matching either typed id form (spec §7)."""
    return isinstance(value, str) and (
        _SECTION_ID_RE.fullmatch(value) is not None or _SOURCE_ID_RE.fullmatch(value) is not None
    )


def id_kind(value: object) -> str | None:
    """'section'/'source' from the id prefix, else None. Case-insensitive."""
    if not isinstance(value, str):
        return None
    if _SECTION_ID_RE.fullmatch(value):
        return "section"
    if _SOURCE_ID_RE.fullmatch(value):
        return "source"
    return None


def normalize_id(value: str) -> str:
    """Casefold key for id comparison (ASCII lower); typed ids are ASCII."""
    return value.translate(_ASCII_LOWER)


def indexed_dicts(value: object) -> list[tuple[int, dict[str, Any]]]:
    """The (original index, item) pairs of a list's dict items; [] for non-lists."""
    if not isinstance(value, list):
        return []
    return [(index, item) for index, item in enumerate(value) if isinstance(item, dict)]


def reference_code(
    value: object,
    expected: str,
    source_ids: frozenset[str],
    section_ids: frozenset[str],
) -> str | None:
    """AKB010 (wrong-kind prefix) / AKB007 (unresolved) / None for one reference.

    Non-typed tokens are skipped: the schema layer already reported them as AKB011,
    and an unresolvable malformed token is one violation, not two. `source_ids` and
    `section_ids` hold normalized (lowercased) ids.
    """
    kind = id_kind(value)
    if kind is None:
        return None
    if kind != expected:
        return "AKB010"
    registry = source_ids if expected == "source" else section_ids
    value = cast("str", value)  # id_kind guarantees str
    return None if normalize_id(value) in registry else "AKB007"
