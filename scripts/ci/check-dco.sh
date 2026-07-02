#!/usr/bin/env bash
# Verify every non-merge commit in BASE..HEAD carries a DCO Signed-off-by trailer.
# Usage: check-dco.sh <base-sha> <head-sha>
set -euo pipefail

base="${1:?base sha required}"
head="${2:?head sha required}"

# Compute the range up front and FAIL CLOSED if git can't (bad/unreachable sha, shallow gap).
# (A `< <(git rev-list …)` process substitution would swallow this error and pass vacuously.)
if ! commits="$(git rev-list "$base..$head")"; then
  echo "::error::could not compute commit range $base..$head"
  exit 1
fi

missing=0
while IFS= read -r sha; do
  [ -z "$sha" ] && continue
  # Skip merge commits (more than one parent).
  parents="$(git rev-list --parents -n 1 "$sha" | wc -w | tr -d ' ')"
  [ "$parents" -gt 2 ] && continue
  # Skip bot-authored commits (mirrors dco.yml's actor exemption), so a
  # maintainer pushing to e.g. a Dependabot branch is not blocked.
  case "$(git log -1 --format='%an' "$sha")" in *'[bot]') continue ;; esac
  signoff="$(git log -1 --format='%(trailers:key=Signed-off-by,valueonly)' "$sha")"
  if [ -z "${signoff//[[:space:]]/}" ]; then
    echo "::error::commit $sha is missing a Signed-off-by line (DCO)"
    missing=1
  fi
done <<< "$commits"

if [ "$missing" -ne 0 ]; then
  echo "One or more commits lack DCO sign-off. Fix with: git commit --amend -s (or git rebase --signoff)."
  exit 1
fi
echo "All commits carry a DCO Signed-off-by line."
