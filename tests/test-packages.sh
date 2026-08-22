#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0
grep -Fq 'package=git|apt:git' "$ROOT/modules/git/module.conf" || failures=$((failures + 1))
grep -Fq 'package=git|winget:Git.Git' "$ROOT/modules/git/module.conf" || failures=$((failures + 1))
grep -Fq -- '--yes' "$ROOT/install.sh" || failures=$((failures + 1))

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin" "$tmp/home"
cat > "$tmp/bin/apt" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
cat > "$tmp/bin/dpkg-query" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
cat > "$tmp/bin/sudo" <<'EOF'
#!/usr/bin/env bash
"$@"
EOF
cat > "$tmp/bin/run0" <<'EOF'
#!/usr/bin/env bash
"$@"
EOF
cat > "$tmp/bin/apt-get" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
chmod +x "$tmp/bin"/*
PATH="$tmp/bin:$PATH" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps git --yes >/dev/null || failures=$((failures + 1))
[ -L "$tmp/home/.gitconfig" ] || { printf 'FAIL: package-enabled module was not installed\n' >&2; failures=$((failures + 1)); }

cat > "$tmp/bin/apt-get" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
rm -f "$tmp/home/.gitconfig"
if PATH="$tmp/bin:$PATH" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-failure" bash "$ROOT/install.sh" --apps git --yes >/dev/null 2>&1; then
    printf 'FAIL: package failure succeeded\n' >&2
    failures=$((failures + 1))
fi
[ ! -e "$tmp/home/.gitconfig" ] || { printf 'FAIL: dotfile installed after package failure\n' >&2; failures=$((failures + 1)); }

[ "$failures" -eq 0 ] || exit 1
printf 'package tests passed\n'
