#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0
assert_contains() { printf '%s' "$1" | grep -Fq -- "$2" || { printf 'FAIL: expected %s\n' "$2" >&2; failures=$((failures + 1)); }; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

grep -Fq 'default=true' "$ROOT/modules/git/module.conf" || failures=$((failures + 1))
grep -Fq 'default=true' "$ROOT/modules/bash/module.conf" || failures=$((failures + 1))
grep -Fq "\$'\\x1b')" "$ROOT/install.sh" || failures=$((failures + 1))
grep -Fq 'q)' "$ROOT/install.sh" || failures=$((failures + 1))

output="$(HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --list)"
assert_contains "$output" 'git - Git configuration'
assert_contains "$output" 'bash - Bash profile'
if printf '%s' "$output" | grep -Fq 'shell -'; then
    printf 'FAIL: legacy shell module is still listed\n' >&2
    failures=$((failures + 1))
fi

mkdir -p "$tmp/home"
printf 'old\n' > "$tmp/home/.bashrc"
HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps bash >/dev/null || failures=$((failures + 1))
[ -L "$tmp/home/.bashrc" ] || { printf 'bashrc was not linked through HOME expansion\n' >&2; failures=$((failures + 1)); }
[ -f "$tmp/state/dotfiles/backups"/*/.bashrc ] || { printf 'FAIL: backup missing\n' >&2; failures=$((failures + 1)); }

if HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps unknown >/dev/null 2>&1; then
    printf 'FAIL: unknown module succeeded\n' >&2
    failures=$((failures + 1))
fi

HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps '' | grep -Fq 'No applications selected.' || failures=$((failures + 1))

[ "$failures" -eq 0 ] || exit 1
printf 'shell installer tests passed\n'
