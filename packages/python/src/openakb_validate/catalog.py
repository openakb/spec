"""Stable error-code catalog and normative caps (spec §7)."""

from __future__ import annotations

__all__ = ["CODE_NAMES", "LOCAL_ID_CHARSET", "LOCAL_ID_MAX_LENGTH", "PARENT_DEPTH_MAX"]

CODE_NAMES: dict[str, str] = {
    "AKB001": "id-not-unique",
    "AKB002": "empty-section",
    "AKB003": "missing-source-cite",
    "AKB004": "parent-cycle",
    "AKB005": "cap-exceeded",
    "AKB006": "unknown-core-property",
    "AKB007": "unresolved-reference",
    "AKB008": "unknown-rel",
    "AKB009": "missing-required-field",
    "AKB010": "invalid-reference-kind",
    "AKB011": "malformed-value",
    "AKB012": "link-missing-target",
}

# The one normative cap the JSON Schema cannot express (spec §7): root sections have
# depth 1; the deepest permitted section has depth 64.
PARENT_DEPTH_MAX = 64

# The local ID grammar (spec §7): every id and inline `[cite:]` id is one or more of
# these characters, capped at this length. Shared so the shape checker and the citation
# extractor read the cap from one place and cannot drift apart.
LOCAL_ID_CHARSET = "a-z0-9_-"
LOCAL_ID_MAX_LENGTH = 64
