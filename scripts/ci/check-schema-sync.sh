#!/usr/bin/env bash
# Validator packages vendor the published schemas (runtime/package resource APIs need
# real files inside the package). This gate keeps those copies byte-identical to schema/v1/.
set -euo pipefail
cd "$(dirname "$0")/../.."

status=0
for dir in packages/python/src/openakb_validate/schemas packages/rust/schemas; do
  for name in openakb.schema.json provenance.schema.json; do
    if ! cmp -s "schema/v1/$name" "$dir/$name"; then
      echo "::error file=$dir/$name::vendored schema is out of sync with schema/v1/$name" >&2
      status=1
    fi
  done
done
exit $status
