#!/usr/bin/env bash
# Update version strings in language-specific manifests to match the given tag.
set -euo pipefail

tag="${1:?usage: update-versions.sh <tag>}"
ver="${tag#v}"

if [ -f Cargo.toml ]; then
  sed -i.bak -E "s/^version = \".*\"/version = \"${ver}\"/" Cargo.toml && rm Cargo.toml.bak
fi
if [ -f package.json ]; then
  sed -i.bak -E "s/(\"version\": \")[^\"]*(\")/\1${ver}\2/" package.json && rm package.json.bak
fi
if [ -f pyproject.toml ]; then
  sed -i.bak -E "s/^version = \".*\"/version = \"${ver}\"/" pyproject.toml && rm pyproject.toml.bak
fi
