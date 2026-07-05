#!/usr/bin/env bash
# The wheel is what users install: prove the vendored schemas are inside it by loading
# both through importlib.resources from a clean environment (not the source tree), so a
# packaging regression can never ship a wheel that fails on first validation.
set -euo pipefail
cd "$(dirname "$0")/../../packages/python"

shopt -s nullglob
wheels=(dist/openakb_validate-*.whl)
if [ "${#wheels[@]}" -ne 1 ]; then
  echo "::error::expected exactly one wheel in packages/python/dist, found ${#wheels[@]}" >&2
  exit 1
fi
wheel="${wheels[0]}"
uv run --isolated --no-project --with "${wheel}" python - <<'PY'
import json
from importlib.resources import files

for name in ("openakb.schema.json", "provenance.schema.json"):
    json.loads(files("openakb_validate.schemas").joinpath(name).read_text("utf-8"))
print("wheel bundles both schemas")
PY
