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
# Pass the secrets as flags instead of piping them on stdin: atuin asks for the
# encryption key before the password on the default (Hub) sync path and in the
# opposite order on the legacy path, and the password prompt reads the terminal
# rather than stdin, so no fixed stdin order satisfies both.
if [ -n "${ATUIN_SYNC_ADDRESS:-}" ]; then
    ATUIN_SYNC_ADDRESS="$ATUIN_SYNC_ADDRESS" atuin login -u "$ATUIN_USERNAME" --password "$ATUIN_PASSWORD" --key "$ATUIN_KEY"
else
    atuin login -u "$ATUIN_USERNAME" --password "$ATUIN_PASSWORD" --key "$ATUIN_KEY"
fi
