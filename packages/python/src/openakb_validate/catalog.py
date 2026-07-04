"""Stable error-code catalog and normative caps (spec §7)."""

from __future__ import annotations

__all__ = ["CODE_NAMES", "PARENT_DEPTH_MAX"]

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
