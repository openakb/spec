"""Internal shape helpers shared by the semantic, strict, and content layers.

Everything here is defensive: descriptors arrive as arbitrary parsed JSON, and type
errors are the schema layer's job, so unexpected shapes are skipped, never raised on.
"""

from __future__ import annotations

import re
from typing import Any

from .catalog import LOCAL_ID_CHARSET, LOCAL_ID_MAX_LENGTH

__all__ = ["LOCAL_ID_RE", "indexed_dicts", "is_local_id", "reference_code"]

LOCAL_ID_RE = re.compile(rf"^[{LOCAL_ID_CHARSET}]{{1,{LOCAL_ID_MAX_LENGTH}}}$")


def is_local_id(value: object) -> bool:
    """True iff value is a string matching the local ID grammar (spec §7)."""
    return isinstance(value, str) and LOCAL_ID_RE.fullmatch(value) is not None


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
    """AKB007/AKB010/None for one local reference token expecting a 'source' or 'section'.

    Tokens failing the local-ID grammar are skipped: the schema layer already reported
    them as AKB011, and an unresolvable malformed token is one violation, not two.
    """
    if not is_local_id(value):
        return None
    resolved = {
        kind for kind, ids in (("source", source_ids), ("section", section_ids)) if value in ids
    }
    if expected in resolved:
        return None
    return "AKB010" if resolved else "AKB007"
