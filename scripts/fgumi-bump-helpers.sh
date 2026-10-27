#!/bin/bash
#
# Version helpers shared by the steps of .github/workflows/fgumi-bump.yml.
#
# Usage: source scripts/fgumi-bump-helpers.sh

# is_version V: V is a plain X.Y.Z version.
is_version() { [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# gt A B: version A is strictly newer than version B.
gt() { [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]; }

# bump_fgumi V: require fgumi V in Cargo.toml and lock exactly V; other locked
# versions move only if V requires it. `cargo add` alone locks the newest
# release compatible with V, which is not V when a dispatch names an older
# release than the latest.
bump_fgumi() {
  cargo add "fgumi@$1" \
    && cargo update -p fgumi --precise "$1" \
    && grep -qx "fgumi = \"$1\"" Cargo.toml
}
