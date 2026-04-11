#!/usr/bin/env bash
# Regenerate CHANGELOG.md from conventional-commit history up to the given tag.
set -euo pipefail

tag="${1:?usage: generate-changelog.sh <tag>}"
prev=$(git describe --tags --abbrev=0 "${tag}^" 2>/dev/null || echo "")

range="${prev:+${prev}..}${tag}"

{
  echo "# Changelog"
  echo
  echo "## [${tag#v}]"
  echo
  git log --pretty=format:'- %s' "$range" | sort -u
  echo
} > CHANGELOG.md
