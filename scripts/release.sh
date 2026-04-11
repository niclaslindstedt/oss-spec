#!/usr/bin/env bash
# Compute next version from conventional-commit history (or accept patch/minor/major).
# Tag-only — no manifest churn. The release workflow handles changelog + version updates.
set -euo pipefail

bump="${1:-auto}"
current=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
current="${current#v}"
IFS=. read -r major minor patch <<<"$current"

case "$bump" in
  auto)
    range="v${current}..HEAD"
    if git log --oneline "$range" 2>/dev/null | grep -qE '(^|: )(BREAKING CHANGE|[a-z]+!:)'; then
      bump=major
    elif git log --oneline "$range" 2>/dev/null | grep -qE '^[a-f0-9]+ feat'; then
      bump=minor
    else
      bump=patch
    fi
    ;;
esac

case "$bump" in
  major) major=$((major+1)); minor=0; patch=0 ;;
  minor) minor=$((minor+1)); patch=0 ;;
  patch) patch=$((patch+1)) ;;
  *) echo "unknown bump: $bump" >&2; exit 1 ;;
esac

version="${major}.${minor}.${patch}"
tag="v${version}"

git tag "$tag"
echo "version=${version}" >> "${GITHUB_OUTPUT:-/dev/null}"
echo "tagged ${tag}"
