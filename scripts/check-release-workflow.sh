#!/usr/bin/env bash
# The release workflow's contract (V105). It runs only on a tag, so nothing
# else exercises it before the release it exists for.
set -euo pipefail

workflow=.github/workflows/release.yml

need() {
  grep -Fq -- "$1" "$workflow" || {
    echo "hk: $workflow no longer holds \`$1\` (V105). Restore it, or change V105 and this check in the same commit." >&2
    exit 1
  }
}

test -f "$workflow" || { echo "hk: $workflow is missing (V105)." >&2; exit 1; }
need 'tags: ["v*"]'
need 'workflow_dispatch:'
need 'tag:'
need 'ref: refs/tags/${{ inputs.tag || github.ref_name }}'
need 'ubuntu-latest'
need 'ubuntu-24.04-arm'
need 'macos-latest'
need 'gh release upload --clobber'
need 'GH_REPO: ${{ github.repository }}'
need 'SHA256SUMS'
need 'LICENSE'
need 'README.md'
need 'release/ctrm --version'
# The compiler the Action pins (V52), so a release builds with what the
# gate tested.
pin="$(sed -n 's/^ *\(RUSTUP_TOOLCHAIN: ".*"\)$/\1/p' action.yml | head -n 1)"
test -n "$pin" || { echo "hk: action.yml pins no RUSTUP_TOOLCHAIN for the release to match (V52)." >&2; exit 1; }
need "$pin"
test "$(grep -Fc 'tar -czf' "$workflow")" -eq 1 || { echo "hk: $workflow must build exactly one archive per target (V105)." >&2; exit 1; }
test "$(grep -Fc 'target:' "$workflow")" -eq 3 || { echo "hk: $workflow must build exactly three targets (V105)." >&2; exit 1; }
