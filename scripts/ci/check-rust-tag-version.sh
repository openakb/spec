#!/usr/bin/env bash
# Publishing is tag-driven: refuse to publish when the rust-v* tag disagrees with
# the crate version, so a mistagged release can never reach crates.io.
set -euo pipefail
cd "$(dirname "$0")/../.."

tag="${1:?usage: check-rust-tag-version.sh <tag> (e.g. rust-v0.1.0)}"
version="$(cargo metadata --manifest-path packages/rust/Cargo.toml --no-deps --format-version 1 | jq -r '.packages[0].version')"
expected="rust-v${version}"
if [ "${expected}" != "${tag}" ]; then
  echo "::error::tag ${tag} does not match crate version ${version}" >&2
  exit 1
fi
echo "tag ${tag} matches crate version ${version}"
