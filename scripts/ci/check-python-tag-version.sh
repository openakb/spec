#!/usr/bin/env bash
# Publishing is tag-driven: refuse to publish when the py-v* tag disagrees with the
# package version, so a mistagged release can never reach PyPI.
set -euo pipefail
cd "$(dirname "$0")/../../packages/python"

tag="${1:?usage: check-python-tag-version.sh <tag> (e.g. py-v0.1.0)}"
version="$(uv run python -c 'import openakb_validate; print(openakb_validate.__version__)')"
if [ "py-v${version}" != "${tag}" ]; then
  echo "::error::tag ${tag} does not match package version ${version}" >&2
  exit 1
fi
echo "tag ${tag} matches package version ${version}"
