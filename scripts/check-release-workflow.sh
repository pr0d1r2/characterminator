#!/usr/bin/env bash
set -euo pipefail

workflow=.github/workflows/release.yml
test -f "$workflow"
grep -Fq 'tags: ["v*"]' "$workflow"
grep -Fq 'workflow_dispatch:' "$workflow"
grep -Fq 'tag:' "$workflow"
grep -Fq 'ref: refs/tags/${{ inputs.tag || github.ref_name }}' "$workflow"
grep -Fq 'ubuntu-latest' "$workflow"
grep -Fq 'ubuntu-24.04-arm' "$workflow"
grep -Fq 'macos-latest' "$workflow"
grep -Fq 'gh release upload --clobber' "$workflow"
grep -Fq 'SHA256SUMS' "$workflow"
grep -Fq 'LICENSE' "$workflow"
grep -Fq 'README.md' "$workflow"
grep -Fq 'release/ctrm --version' "$workflow"
test "$(grep -Fc 'tar -czf' "$workflow")" -eq 1
test "$(grep -Fc 'target:' "$workflow")" -eq 3
