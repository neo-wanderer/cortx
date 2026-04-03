#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.4.0
#
# This script:
#   1. Validates the version argument
#   2. Checks the working tree is clean and on main
#   3. Bumps version in Cargo.toml and regenerates Cargo.lock
#   4. Commits the version bump
#   5. Creates and pushes the tag (triggers CI release workflow)

VERSION="${1-}"

if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.4.0"
  exit 1
fi

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: version must be in semver format (e.g. 0.4.0)"
  exit 1
fi

TAG="v$VERSION"

# Must be on main
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$BRANCH" != "main" ]]; then
  echo "Error: must be on main branch (currently on '$BRANCH')"
  exit 1
fi

# Working tree must be clean
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree has uncommitted changes"
  exit 1
fi

# Tag must not already exist
if git rev-parse "$TAG" &>/dev/null; then
  echo "Error: tag $TAG already exists"
  exit 1
fi

echo "Releasing $TAG..."

# Bump Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Regenerate Cargo.lock
cargo update --workspace --quiet

# Commit
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $TAG"

# Tag and push
git tag "$TAG"
git push origin main
git push origin "$TAG"

echo ""
echo "Done. Tag $TAG pushed — CI will build binaries and create the GitHub release."
