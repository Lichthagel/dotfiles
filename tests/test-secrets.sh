#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if grep -Fq 'secret=' "$ROOT/modules/atuin/module.conf"; then failures=$((failures + 1)); fi
grep -Fq 'setup=linux:atuin/setup.sh' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'map=linux:atuin/bash/50-atuin.bash|${HOME}/.config/bashrc.d/50-atuin.bash|requires=bash' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'run_module_setup' "$ROOT/install.sh" || failures=$((failures + 1))
grep -Fq 'secrets/${module}.env.age' "$ROOT/install.sh" || failures=$((failures + 1))
grep -Fq 'modules/dependencies.conf' "$ROOT/install.sh" || failures=$((failures + 1))
grep -Fq 'Age identity file path' "$ROOT/lib/secrets.sh" || failures=$((failures + 1))
if grep -Fq 'process_atuin_secrets' "$ROOT/install.sh" || grep -Fq 'atuin login' "$ROOT/install.sh" || grep -Fq 'ATUIN_USERNAME' "$ROOT/install.sh"; then failures=$((failures + 1)); fi
if grep -Fq 'Process-AtuinSecrets' "$ROOT/install.ps1" || grep -Fq 'atuin login' "$ROOT/install.ps1" || grep -Fq 'ATUIN_USERNAME' "$ROOT/install.ps1"; then failures=$((failures + 1)); fi
if grep -Fq 'secrets.sh' "$ROOT/modules/atuin/setup.sh" || grep -Fq 'dotfiles_decrypt_env' "$ROOT/modules/atuin/setup.sh" || grep -Fq 'age --decrypt' "$ROOT/modules/atuin/setup.sh"; then failures=$((failures + 1)); fi
if grep -Fq 'secrets.ps1' "$ROOT/modules/atuin/setup.ps1" || grep -Fq 'Initialize-DotfilesSecrets' "$ROOT/modules/atuin/setup.ps1" || grep -Fq 'age --decrypt' "$ROOT/modules/atuin/setup.ps1"; then failures=$((failures + 1)); fi
grep -Fq 'setup=linux:opencode/setup.sh' "$ROOT/modules/opencode/module.conf" || failures=$((failures + 1))
grep -Fq 'setup=windows:opencode/setup.ps1' "$ROOT/modules/opencode/module.conf" || failures=$((failures + 1))
grep -Fq 'setup=*) SETUPS+=' "$ROOT/install.sh" || failures=$((failures + 1))
grep -Fq 'setup=(linux|windows):' "$ROOT/install.ps1" || failures=$((failures + 1))
for installer in "$ROOT/install.sh" "$ROOT/install.ps1"; do
    if grep -Eiq 'opencode|openrouter|azure_api_key|digitalocean|theme|mcp|plugin' "$installer"; then
        printf 'FAIL: installer contains OpenCode-specific setup logic: %s\n' "$installer" >&2
        failures=$((failures + 1))
    fi
done

mkdir -p "$tmp/bin" "$tmp/home/.config/age" "$tmp/state" "$tmp/log"
printf 'identity\n' > "$tmp/home/.config/age/keys.txt"
printf 'encrypted fixture\n' > "$tmp/atuin.env.age"
cat > "$tmp/bin/age" <<'EOF'
#!/usr/bin/env bash
if [ "${ATUIN_TEST_LOG_IDENTITY:-0}" = 1 ]; then cat "$3" > "$ATUIN_TEST_LOG/identity"; fi
cat "${@: -1}"
EOF
cat > "$tmp/bin/atuin" <<'EOF'
#!/usr/bin/env bash
if [ "$1" = status ]; then
    [ "${ATUIN_TEST_LOGGED_IN:-0}" = 1 ] && exit 0 || exit 1
fi
printf '%s\n' "$*" > "$ATUIN_TEST_LOG/args"
cat > "$ATUIN_TEST_LOG/stdin"
exit 0
EOF
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
printf '%s\n' "$*" >> "$ATUIN_TEST_LOG/package-installs"
exit 0
EOF
chmod +x "$tmp/bin"/*

cat > "$tmp/atuin.env.age" <<'EOF'
ATUIN_USERNAME=test-user
ATUIN_PASSWORD=test-password
ATUIN_KEY=test-key
EOF
mkdir -p "$tmp/log"
PATH="$tmp/bin:$PATH" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" ATUIN_TEST_LOG="$tmp/log" DOTFILES_SECRETS_FILE="$tmp/atuin.env.age" bash "$ROOT/install.sh" --apps atuin --yes >/dev/null || failures=$((failures + 1))

grep -Fq 'age' "$tmp/log/package-installs" || failures=$((failures + 1))
grep -Fxq 'login -u test-user' "$tmp/log/args" || failures=$((failures + 1))
printf 'test-password\ntest-key\n' | cmp -s - "$tmp/log/stdin" || failures=$((failures + 1))
[ ! -e "$tmp/state/dotfiles/secrets" ] || failures=$((failures + 1))

mkdir -p "$tmp/bash-only-home"
PATH="$tmp/bin:$PATH" HOME="$tmp/bash-only-home" XDG_STATE_HOME="$tmp/bash-only-state" bash "$ROOT/install.sh" --apps bash --yes >/dev/null || failures=$((failures + 1))
[ ! -e "$tmp/bash-only-home/.config/bashrc.d/50-atuin.bash" ] || failures=$((failures + 1))

mkdir -p "$tmp/bash-atuin-home"
mkdir -p "$tmp/bash-atuin-home/.config/age"
printf 'identity\n' > "$tmp/bash-atuin-home/.config/age/keys.txt"
PATH="$tmp/bin:$PATH" HOME="$tmp/bash-atuin-home" XDG_STATE_HOME="$tmp/bash-atuin-state" ATUIN_TEST_LOG="$tmp/log" DOTFILES_SECRETS_FILE="$tmp/atuin.env.age" bash "$ROOT/install.sh" --apps bash,atuin --yes >/dev/null || failures=$((failures + 1))
grep -Fxq 'eval "$(atuin init bash)"' "$tmp/bash-atuin-home/.config/bashrc.d/50-atuin.bash" || failures=$((failures + 1))

rm -f "$tmp/log/args" "$tmp/log/stdin"
PATH="$tmp/bin:$PATH" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-logged-in" ATUIN_TEST_LOG="$tmp/log" ATUIN_TEST_LOGGED_IN=1 DOTFILES_SECRETS_FILE="$tmp/atuin.env.age" bash "$ROOT/install.sh" --apps atuin --yes >/dev/null || failures=$((failures + 1))
[ ! -e "$tmp/log/args" ] || failures=$((failures + 1))

rm -f "$tmp/log/args" "$tmp/log/stdin" "$tmp/log/identity"
PATH="$tmp/bin:$PATH" HOME="$tmp/no-key-home" XDG_STATE_HOME="$tmp/state-direct" ATUIN_TEST_LOG="$tmp/log" ATUIN_TEST_LOG_IDENTITY=1 AGE_IDENTITY='direct-age-key' DOTFILES_SECRETS_FILE="$tmp/atuin.env.age" bash "$ROOT/install.sh" --apps atuin --yes >/dev/null || failures=$((failures + 1))
grep -Fxq 'direct-age-key' "$tmp/log/identity" || failures=$((failures + 1))

printf 'ATUIN_USERNAME=test-user\nATUIN_PASSWORD=test-password\n' > "$tmp/incomplete.env.age"
if PATH="$tmp/bin:$PATH" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-missing" ATUIN_TEST_LOG="$tmp/log" DOTFILES_SECRETS_FILE="$tmp/incomplete.env.age" bash "$ROOT/install.sh" --apps atuin --yes >/dev/null 2>&1; then
    failures=$((failures + 1))
fi

# A piped one-liner has a non-terminal stdin but still has a controlling TTY.
# The optional secret flow must prompt on that TTY instead of skipping the bundle.
if command -v script >/dev/null 2>&1; then
    mkfifo "$tmp/secret-input"
    printf 'fixture-age-key\n' > "$tmp/secret-input" &
    secret_prompt_output="$(script -qec "bash -c 'exec 0<\"$tmp/secret-input\"; export HOME=\"$tmp/no-key-home\"; source \"$ROOT/lib/secrets.sh\"; DOTFILES_SECRET_FILE=\"$tmp/atuin.env.age\" DOTFILES_SECRETS_OPTIONAL=1 dotfiles_decrypt_env; printf \"%s\\n\" \"\$ATUIN_USERNAME\"'" /dev/null 2>&1)"
    wait
    printf '%s' "$secret_prompt_output" | grep -Fq 'Age identity file path or key' || failures=$((failures + 1))
    printf '%s' "$secret_prompt_output" | grep -Fq 'test-user' || failures=$((failures + 1))
fi

[ "$failures" -eq 0 ] || exit 1
printf 'secret tests passed\n'
