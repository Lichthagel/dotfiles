#!/usr/bin/env bash
set -u

if command -v brew >/dev/null 2>&1; then
    exit 0
fi

command -v curl >/dev/null 2>&1 || { printf 'curl is required to install Homebrew.\n' >&2; exit 2; }
NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || exit 1

for brew_prefix in /home/linuxbrew/.linuxbrew /home/linuxbrew/.linuxbrew/bin; do
    [ -x "$brew_prefix/bin/brew" ] && export PATH="$brew_prefix/bin:$brew_prefix/sbin:$PATH"
done

command -v brew >/dev/null 2>&1 || { printf 'Homebrew was installed but is not available on PATH.\n' >&2; exit 1; }
eval "$(brew shellenv)"
