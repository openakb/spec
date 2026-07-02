#!/usr/bin/env bash
# Fail if a forbidden vendor/product name appears in a tracked PUBLIC artifact (D26). Gitignored
# internal working state is outside tracked-file `git grep`. This script excludes ITSELF (its own
# name list would self-match) and honors the one carve-out: GOVERNANCE.md MAY name the first
# implementer per AGENTS.md/D26. Prior-art hyperlinks are permitted and are not listed here.
set -uo pipefail

self='scripts/ci/check-neutrality.sh'
found=0

# Forbidden everywhere (no carve-out):
if git grep -nEi 'LangChain|LangGraph' -- ":!$self"; then found=1; fi
# The first-implementer name is carved out for GOVERNANCE.md only:
if git grep -nEi 'Nurok' -- ":!GOVERNANCE.md" ":!$self"; then found=1; fi

if [ "$found" -ne 0 ]; then
  echo "::error::forbidden vendor/product name found in a public artifact (D26); see matches above."
  exit 1
fi
echo "Neutrality OK: no forbidden vendor names in public artifacts."
