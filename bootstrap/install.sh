#!/usr/bin/env bash
set -eu

case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64) ;;
    *) printf 'This bootstrap supports Linux x86_64 only.\n' >&2; exit 2 ;;
esac

repo_url="${DOTFILES_REPO_URL:-https://github.com/Lichthagel/dotfiles}"
version="${DOTFILES_VERSION:-latest}"
repo_url="${repo_url%/}"
if [ "$version" = latest ]; then
    release_url="$repo_url/releases/latest/download"
else
    release_url="$repo_url/releases/download/$version"
fi

temporary_dir="$(mktemp -d "${TMPDIR:-/tmp}/dotfiles-bootstrap.XXXXXX")"
cleanup() { rm -rf -- "$temporary_dir"; }
trap cleanup EXIT

binary="$temporary_dir/dotfiles-linux-x86_64"
checksum="$temporary_dir/dotfiles-linux-x86_64.sha256"
curl -fsSL "$release_url/dotfiles-linux-x86_64" -o "$binary"
curl -fsSL "$release_url/dotfiles-linux-x86_64.sha256" -o "$checksum"

expected=''
while IFS=' ' read -r candidate _; do
    if [ -n "$candidate" ]; then
        expected="$(printf '%s' "$candidate" | tr '[:upper:]' '[:lower:]')"
        break
    fi
done < "$checksum"
if [ -z "$expected" ]; then
    printf 'Release checksum is empty.\n' >&2
    exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$binary")"
    actual="${actual%% *}"
    actual="$(printf '%s' "$actual" | tr '[:upper:]' '[:lower:]')"
elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$binary")"
    actual="${actual%% *}"
    actual="$(printf '%s' "$actual" | tr '[:upper:]' '[:lower:]')"
else
    printf 'Neither sha256sum nor shasum is available.\n' >&2
    exit 1
fi
if [ "$actual" != "$expected" ]; then
    printf 'Release checksum verification failed.\n' >&2
    exit 1
fi
printf 'verified dotfiles-linux-x86_64\n'
chmod +x "$binary"

if [ -t 1 ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
    "$binary" "$@" </dev/tty
else
    "$binary" "$@"
fi
status=$?
exit "$status"
