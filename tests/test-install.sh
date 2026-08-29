#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tests/test-helpers.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

grep -Fq 'default=true' "$ROOT/modules/git/module.conf" || fail_test 'git default missing'
grep -Fq 'default=true' "$ROOT/modules/bash/module.conf" || fail_test 'bash default missing'
grep -Fq 'for file in "$HOME/.config/bashrc.d/"*.bash' "$ROOT/modules/bash/bashrc" || fail_test 'Bash drop-in loader missing'
grep -Fq 'atuin init' "$ROOT/modules/bash/bashrc" && fail_test 'Bash profile contains Atuin-specific initialization'
grep -Fq "\$'\\x1b')" "$ROOT/install.sh" || fail_test 'Bash cancellation handling missing'
grep -Fq 'q)' "$ROOT/install.sh" || fail_test 'Bash q cancellation handling missing'
grep -Fq 'DOTFILES_REPO_URL:-https://github.com/Lichthagel/dotfiles' "$ROOT/install.sh" || fail_test 'Bash piped bootstrap default missing'
grep -Fq 'exec bash "$extracted/install.sh"' "$ROOT/install.sh" || fail_test 'Bash piped bootstrap exec fallback missing'
grep -Fq 'curl -fsSL https://raw.githubusercontent.com/Lichthagel/dotfiles/main/install.sh | sh' "$ROOT/README.md" || fail_test 'Bash published one-liner missing'
grep -Fq 'irm https://raw.githubusercontent.com/Lichthagel/dotfiles/main/install.ps1 | iex' "$ROOT/README.md" || fail_test 'PowerShell published one-liner missing'

if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    read -r install_mode _ <<EOF
$(git -C "$ROOT" ls-files -s install.sh)
EOF
    [ "$install_mode" = 100755 ] || fail_test 'install.sh is not executable in the git index'
    for setup_script in $(grep -h '^setup=' "$ROOT"/modules/*/module.conf | sed 's/^[^:]*://'); do
        case "$setup_script" in
            *.sh)
                read -r setup_mode _ <<EOF
$(git -C "$ROOT" ls-files -s "modules/$setup_script")
EOF
                [ "$setup_mode" = 100755 ] || fail_test "setup script is not executable in the git index: $setup_script"
                ;;
        esac
    done
fi

output="$(HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --list)"
assert_contains "$output" 'git - Git configuration'
assert_contains "$output" 'bash - Bash profile'
assert_contains "$output" 'oh-my-posh - Oh My Posh prompt'
if printf '%s' "$output" | grep -Fq 'shell -'; then
    fail_test 'legacy shell module is still listed'
fi

help_output="$(bash "$ROOT/install.sh" --help)"
assert_contains "$help_output" 'Usage: install.sh'
if bash "$ROOT/install.sh" --apps >/dev/null 2>&1; then
    fail_test 'missing --apps value succeeded'
fi
if bash "$ROOT/install.sh" --unknown >/dev/null 2>&1; then
    fail_test 'unknown option succeeded'
fi

mkdir -p "$tmp/home"
printf 'old\n' > "$tmp/home/.bashrc"
HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps bash >/dev/null || fail_test 'Bash-only installation failed'
[ -L "$tmp/home/.bashrc" ] || fail_test 'Bashrc was not linked through HOME expansion'
assert_file_not_exists "$tmp/home/.config/bashrc.d/50-atuin.bash"
assert_file_not_exists "$tmp/home/.config/bashrc.d/10-oh-my-posh.bash"
compgen -G "$tmp/state/dotfiles/backups/*/.bashrc" >/dev/null || fail_test 'backup missing'

if HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps unknown >/dev/null 2>&1; then
    fail_test 'unknown module succeeded'
fi

HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" bash "$ROOT/install.sh" --apps '' | grep -Fq 'No applications selected.' || fail_test 'empty app selection was not a no-op'

mkdir -p "$tmp/oh-my-posh-home"
mkdir -p "$tmp/bin"
cat > "$tmp/bin/apt" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
cat > "$tmp/bin/dpkg-query" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
cat > "$tmp/bin/apt-get" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
cat > "$tmp/bin/sudo" <<'EOF'
#!/usr/bin/env bash
"$@"
EOF
cat > "$tmp/bin/run0" <<'EOF'
#!/usr/bin/env bash
"$@"
EOF
chmod +x "$tmp/bin"/*
PATH="$tmp/bin:$PATH" HOME="$tmp/oh-my-posh-home" XDG_STATE_HOME="$tmp/oh-my-posh-state" bash "$ROOT/install.sh" --apps bash,oh-my-posh --yes >/dev/null || fail_test 'Oh My Posh installation failed'
assert_file_exists "$tmp/oh-my-posh-home/.config/oh-my-posh/config.json"
grep -Fq 'oh-my-posh init bash' "$tmp/oh-my-posh-home/.config/bashrc.d/10-oh-my-posh.bash" || fail_test 'Oh My Posh Bash integration missing'

[ "$test_failures" -eq 0 ] || exit 1
printf 'shell installer tests passed\n'
