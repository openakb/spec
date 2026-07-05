#!/usr/bin/env bash
# The Python package vendors the published schemas (importlib.resources needs real files
# inside the package). This gate keeps the vendored copies byte-identical to schema/v1/.
set -euo pipefail
cd "$(dirname "$0")/../.."

status=0
for name in openakb.schema.json provenance.schema.json; do
  if ! cmp -s "schema/v1/$name" "packages/python/src/openakb_validate/schemas/$name"; then
    echo "::error file=packages/python/src/openakb_validate/schemas/$name::vendored schema is out of sync with schema/v1/$name" >&2
    status=1
  fi
done
exit $status
