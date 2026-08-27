#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0

manifest="$ROOT/modules/manifest.conf"
module="$ROOT/modules/opencode/module.conf"

grep -Fq 'module=opencode' "$manifest" || failures=$((failures + 1))
grep -Fq 'name=opencode' "$module" || failures=$((failures + 1))
grep -Fq 'platforms=linux,windows' "$module" || failures=$((failures + 1))
grep -Fq 'default=false' "$module" || failures=$((failures + 1))
grep -Fq 'setup=linux:opencode/setup.sh' "$module" || failures=$((failures + 1))
grep -Fq 'setup=windows:opencode/setup.ps1' "$module" || failures=$((failures + 1))

grep -Fq 'package=opencode|brew:anomalyco/tap/opencode' "$module" || failures=$((failures + 1))
grep -Fq 'package=opencode|mise:github:anomalyco/opencode' "$module" || failures=$((failures + 1))
grep -Fq 'package=opencode|scoop:opencode' "$module" || failures=$((failures + 1))

grep -Fq 'package=jq|apt:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|dnf:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|pacman:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|brew:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|winget:jqlang.jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|scoop:jq' "$module" || failures=$((failures + 1))

[ "$failures" -eq 0 ] || exit 1
printf 'OpenCode Bash assertions passed\n'
