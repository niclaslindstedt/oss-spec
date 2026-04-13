#!/usr/bin/env bash
set -euo pipefail

# Update version strings across all project files.
#
# Usage: update-versions.sh <version>
#
# Updates the version in Cargo.toml and regenerates Cargo.lock.

die() { echo "error: $*" >&2; exit 1; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

VERSION="${1:-}"
[ -n "$VERSION" ] || die "usage: update-versions.sh <version>"

# Validate semver
echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' || die "invalid semver: $VERSION"

echo "updating all version files to $VERSION"

# --- Cargo.toml ---

sed -i.bak "0,/^version = \".*\"/s//version = \"$VERSION\"/" Cargo.toml && rm Cargo.toml.bak
echo "  updated Cargo.toml"

# --- Regenerate Cargo.lock ---

echo "  regenerating Cargo.lock..."
cargo generate-lockfile 2>/dev/null || cargo check 2>/dev/null || true

echo "all versions updated to $VERSION"
