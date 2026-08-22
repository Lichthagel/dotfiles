#!/usr/bin/env bash
set -u
if atuin status >/dev/null 2>&1; then
    printf 'Atuin is already logged in; skipping login.\n'
    exit 0
fi
if [ -z "${ATUIN_USERNAME:-}" ] || [ -z "${ATUIN_PASSWORD:-}" ] || [ -z "${ATUIN_KEY:-}" ]; then
    printf 'Atuin login secrets are missing.\n' >&2
    exit 1
fi
if [ -n "${ATUIN_SYNC_ADDRESS:-}" ]; then
    printf '%s\n%s\n' "$ATUIN_PASSWORD" "$ATUIN_KEY" | ATUIN_SYNC_ADDRESS="$ATUIN_SYNC_ADDRESS" atuin login -u "$ATUIN_USERNAME"
else
    printf '%s\n%s\n' "$ATUIN_PASSWORD" "$ATUIN_KEY" | atuin login -u "$ATUIN_USERNAME"
fi
