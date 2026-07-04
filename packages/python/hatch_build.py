"""Sdist build customization for ``openakb-validate``.

Hatchling's sdist builder force-includes the repository's VCS ignore file
(``.gitignore``) so an unpacked sdist reproduces the same build exclusions. That
file is a contributor-only artifact -- it names internal working paths that have
no place in a published distribution -- and the force-include bypasses the
``exclude`` list, so it cannot be dropped through configuration alone. This hook
removes it from the sdist, leaving only source, tests, packaging, and the
license. It is scoped to the sdist target and never runs for the wheel; if a
future hatchling stops force-including the ignore file, the hook simply finds
nothing to remove.
"""

from __future__ import annotations

import os
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface

__all__ = ["VcsIgnoreStripHook"]

# Basenames of the version-control ignore files hatchling force-includes.
_VCS_IGNORE_NAMES = frozenset({".gitignore", ".hgignore"})


class VcsIgnoreStripHook(BuildHookInterface):
    """Drop force-included VCS ignore files from the built sdist."""

    PLUGIN_NAME = "vcs-ignore-strip"

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        force_include: dict[str, str] = build_data.get("force_include", {})
        stale = [
            source
            for source, target in force_include.items()
            if os.path.basename(target) in _VCS_IGNORE_NAMES
        ]
        for source in stale:
            del force_include[source]
