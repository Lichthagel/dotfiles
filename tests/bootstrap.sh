#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0

fail_test() {
    printf 'FAIL: %s\n' "$1" >&2
    failures=$((failures + 1))
}

assert_contains() {
    local value="$1" expected="$2"
    printf '%s' "$value" | grep -Fq -- "$expected" || fail_test "expected output to contain: $expected"
}

assert_file_not_exists() {
    [ ! -e "$1" ] && [ ! -L "$1" ] || fail_test "expected file not to exist: $1"
}

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
release="$fixture/repo/releases/download/v1"
mkdir -p "$fixture/bin" "$release"

cat > "$release/dotfiles-linux-x86_64" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" > "${BOOTSTRAP_RECORD:?}"
printf '%s\n' "$0" >> "${BOOTSTRAP_RECORD:?}"
exit 7
EOF
chmod +x "$release/dotfiles-linux-x86_64"
sha256sum "$release/dotfiles-linux-x86_64" | sed 's#  .*#  dotfiles-linux-x86_64#' > "$release/dotfiles-linux-x86_64.sha256"

cat > "$fixture/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -eu
output=''
url=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o) output="$2"; shift 2 ;;
        -*) shift ;;
        *) url="$1"; shift ;;
    esac
done
case "$url" in
    file://*) source="${url#file://}" ;;
    *) printf 'unexpected URL: %s\n' "$url" >&2; exit 2 ;;
esac
[ -n "$output" ] || { printf 'missing output path\n' >&2; exit 2; }
cp -- "$source" "$output"
EOF
chmod +x "$fixture/bin/curl"
cat > "$fixture/bin/uname" <<'EOF'
#!/usr/bin/env bash
case "$1" in
    -s) printf 'Linux\n' ;;
    -m) printf 'x86_64\n' ;;
    *) exit 2 ;;
esac
EOF
chmod +x "$fixture/bin/uname"

record="$fixture/record"
output="$(PATH="$fixture/bin:$PATH" DOTFILES_REPO_URL="file://$fixture/repo" DOTFILES_VERSION=v1 \
    BOOTSTRAP_RECORD="$record" sh "$ROOT/bootstrap/install.sh" --modules git --yes 2>&1)"
status=$?
[ "$status" -eq 7 ] || fail_test "bootstrap did not forward child status (got $status)"
assert_contains "$(cat "$record" 2>/dev/null || true)" '--modules git --yes'
recorded_binary="$(sed -n '2p' "$record" 2>/dev/null || true)"
assert_file_not_exists "$recorded_binary"
assert_contains "$output" 'verified'

printf 'not-a-valid-checksum  dotfiles-linux-x86_64\n' > "$release/dotfiles-linux-x86_64.sha256"
rm -f "$record"
if PATH="$fixture/bin:$PATH" DOTFILES_REPO_URL="file://$fixture/repo" DOTFILES_VERSION=v1 \
    BOOTSTRAP_RECORD="$record" sh "$ROOT/bootstrap/install.sh" --modules git --yes >/dev/null 2>&1; then
    fail_test 'checksum failure succeeded'
fi
assert_file_not_exists "$record"

[ "$failures" -eq 0 ] || exit 1
printf 'bootstrap Bash tests passed\n'
