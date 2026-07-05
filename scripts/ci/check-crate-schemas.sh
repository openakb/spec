#!/usr/bin/env bash
# The .crate is what users install: prove it bundles the vendored schemas
# byte-identical to schema/v1/, so packaging regressions cannot ship.
set -euo pipefail
cd "$(dirname "$0")/../.."

rm -f packages/rust/target/package/*.crate
(cd packages/rust && cargo package --locked --quiet)

shopt -s nullglob
crates=(packages/rust/target/package/*.crate)
if [ "${#crates[@]}" -ne 1 ]; then
  echo "::error::expected exactly one packaged crate in packages/rust/target/package, found ${#crates[@]}" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

tar -xzf "${crates[0]}" -C "$tmp"
roots=("$tmp"/*)
if [ "${#roots[@]}" -ne 1 ] || [ ! -d "${roots[0]}" ]; then
  echo "::error::expected packaged crate to extract to exactly one directory" >&2
  exit 1
fi

status=0
for name in openakb.schema.json provenance.schema.json; do
  if ! cmp -s "schema/v1/${name}" "${roots[0]}/schemas/${name}"; then
    echo "::error file=${roots[0]}/schemas/${name}::packaged schema differs from schema/v1/${name}" >&2
    status=1
  fi
done
exit "${status}"
