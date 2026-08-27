#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0
grep -Fq 'module=atuin' "$ROOT/modules/manifest.conf" || failures=$((failures + 1))
grep -Fq 'module=oh-my-posh' "$ROOT/modules/manifest.conf" || failures=$((failures + 1))
grep -Fq 'module=mise' "$ROOT/modules/manifest.conf" || failures=$((failures + 1))
grep -Fq 'module=brew' "$ROOT/modules/manifest.conf" || failures=$((failures + 1))
grep -Fq 'module=scoop' "$ROOT/modules/manifest.conf" || failures=$((failures + 1))
grep -Fq 'name=scoop' "$ROOT/modules/scoop/module.conf" || failures=$((failures + 1))
grep -Fq 'platforms=windows' "$ROOT/modules/scoop/module.conf" || failures=$((failures + 1))
grep -Fq 'default=false' "$ROOT/modules/scoop/module.conf" || failures=$((failures + 1))
grep -Fq 'provides=scoop' "$ROOT/modules/scoop/module.conf" || failures=$((failures + 1))
grep -Fq 'setup=windows:scoop/setup.ps1' "$ROOT/modules/scoop/module.conf" || failures=$((failures + 1))
grep -Fq 'map=windows:scoop/powershell/05-scoop.ps1|$PROFILE\..\Profile.d\05-scoop.ps1|requires=powershell' "$ROOT/modules/scoop/module.conf" || failures=$((failures + 1))
grep -Fq 'get.scoop.sh' "$ROOT/modules/scoop/setup.ps1" || failures=$((failures + 1))
grep -Fq 'scoop\shims' "$ROOT/modules/scoop/powershell/05-scoop.ps1" || failures=$((failures + 1))
grep -Fq 'name=brew' "$ROOT/modules/brew/module.conf" || failures=$((failures + 1))
grep -Fq 'platforms=linux' "$ROOT/modules/brew/module.conf" || failures=$((failures + 1))
grep -Fq 'default=false' "$ROOT/modules/brew/module.conf" || failures=$((failures + 1))
grep -Fq 'provides=brew' "$ROOT/modules/brew/module.conf" || failures=$((failures + 1))
grep -Fq 'setup=linux:brew/setup.sh' "$ROOT/modules/brew/module.conf" || failures=$((failures + 1))
grep -Fq 'map=linux:brew/bash/05-brew.bash|${HOME}/.config/bashrc.d/05-brew.bash|requires=bash' "$ROOT/modules/brew/module.conf" || failures=$((failures + 1))
grep -Fq 'brew shellenv' "$ROOT/modules/brew/setup.sh" || failures=$((failures + 1))
grep -Fq 'brew shellenv' "$ROOT/modules/brew/bash/05-brew.bash" || failures=$((failures + 1))
grep -Fq 'name=mise' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
grep -Fq 'default=true' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
grep -Fq 'provides=mise' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
if grep -Fq 'mise_required' "$ROOT/install.sh" || grep -Fq 'install_mise' "$ROOT/install.sh"; then failures=$((failures + 1)); fi
grep -Fq 'package=mise|apt:mise' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
grep -Fq 'package=mise|winget:jdx.mise' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
grep -Fq 'map=linux:mise/bash/10-mise.bash|${HOME}/.config/bashrc.d/10-mise.bash|requires=bash' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
grep -Fq 'map=windows:mise/powershell/10-mise.ps1|$PROFILE\..\Profile.d\10-mise.ps1|requires=powershell' "$ROOT/modules/mise/module.conf" || failures=$((failures + 1))
grep -Fq 'mise activate bash' "$ROOT/modules/mise/bash/10-mise.bash" || failures=$((failures + 1))
grep -Fq 'mise activate pwsh' "$ROOT/modules/mise/powershell/10-mise.ps1" || failures=$((failures + 1))
grep -Fq 'name=atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'description=Atuin shell history' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'platforms=linux,windows' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'default=false' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=git|apt:git' "$ROOT/modules/git/module.conf" || failures=$((failures + 1))
grep -Fq 'package=git|winget:Git.Git' "$ROOT/modules/git/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|apt:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|dnf:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|pacman:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|brew:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|mise:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|winget:Atuinsh.Atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'package=atuin|scoop:atuin' "$ROOT/modules/atuin/module.conf" || failures=$((failures + 1))
grep -Fq 'platforms=linux,windows' "$ROOT/modules/oh-my-posh/module.conf" || failures=$((failures + 1))
grep -Fq 'default=false' "$ROOT/modules/oh-my-posh/module.conf" || failures=$((failures + 1))
grep -Fq 'package=oh-my-posh|apt:oh-my-posh' "$ROOT/modules/oh-my-posh/module.conf" || failures=$((failures + 1))
grep -Fq 'package=oh-my-posh|winget:JanDeDobbeleer.OhMyPosh' "$ROOT/modules/oh-my-posh/module.conf" || failures=$((failures + 1))
grep -Fq 'dependency=age|apt:age' "$ROOT/modules/dependencies.conf" || failures=$((failures + 1))
grep -Fq 'dependency=age|winget:FiloSottile.age' "$ROOT/modules/dependencies.conf" || failures=$((failures + 1))
grep -Fq -- '--yes' "$ROOT/install.sh" || failures=$((failures + 1))

# Keep the OpenCode package/setup contract isolated from the host environment.
opencode_fixture="$(mktemp -d)"
opencode_log="$opencode_fixture/events.log"
mkdir -p "$opencode_fixture/bin" "$opencode_fixture/home" "$opencode_fixture/config" "$opencode_fixture/state"
cat > "$opencode_fixture/bin/brew" <<'EOF'
#!/usr/bin/env bash
set -u
case "$1 ${2:-} ${3:-}" in
    'list --versions '* )
        case "${3:-}" in
            anomalyco/tap/opencode) [ -e "$OPENCODE_FIXTURE_BIN/opencode" ] ;;
            jq) [ -e "$OPENCODE_FIXTURE_BIN/jq" ] ;;
            *) exit 1 ;;
        esac
        exit $?
        ;;
    'install '* )
        printf 'package:%s\n' "$2" >> "$OPENCODE_FIXTURE_LOG"
        case "$2" in
            jq) printf '#!/usr/bin/env bash\nprintf "setup:jq\\n" >> "$OPENCODE_FIXTURE_LOG"\nprintf "{}\\n"\n' > "$OPENCODE_FIXTURE_BIN/jq"; chmod +x "$OPENCODE_FIXTURE_BIN/jq" ;;
            anomalyco/tap/opencode) printf '#!/usr/bin/env bash\nexit 0\n' > "$OPENCODE_FIXTURE_BIN/opencode"; chmod +x "$OPENCODE_FIXTURE_BIN/opencode" ;;
        esac
        ;;
esac
EOF
cat > "$opencode_fixture/bin/dpkg-query" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
cat > "$opencode_fixture/bin/sudo" <<'EOF'
#!/usr/bin/env bash
"$@"
EOF
cat > "$opencode_fixture/bin/run0" <<'EOF'
#!/usr/bin/env bash
"$@"
EOF
cat > "$opencode_fixture/bin/age" <<'EOF'
#!/usr/bin/env bash
printf 'OPENROUTER_API_KEY=test-openrouter\nAZURE_API_KEY=test-azure\nAZURE_RESOURCE_NAME=test-resource\nDIGITALOCEAN_ACCESS_TOKEN=test-digitalocean\n'
EOF
cat > "$opencode_fixture/bin/curl" <<'EOF'
#!/usr/bin/env bash
case "$*" in
    *tree.json*) printf '{"tree":[{"path":"themes/test.json","type":"blob"}]}\n' ;;
    *) printf '{"theme":"test"}\n' ;;
esac
EOF
cat > "$opencode_fixture/bin/mktemp" <<'EOF'
#!/usr/bin/env bash
exec /usr/bin/mktemp "$@"
EOF
chmod +x "$opencode_fixture/bin"/*
for command in bash cat chmod cp date dirname env find grep head ln mkdir mv rm sort awk stat wc tail cut; do
    command_path="$(type -P "$command" || true)"
    [ -n "$command_path" ] && ln -s "$command_path" "$opencode_fixture/bin/$command"
done
printf 'encrypted fixture\n' > "$opencode_fixture/secrets.env.age"

# A pre-existing command is reported as installed by the fake manager.
printf '#!/usr/bin/env bash\nexit 0\n' > "$opencode_fixture/bin/opencode"
chmod +x "$opencode_fixture/bin/opencode"
: > "$opencode_log"
PATH="$opencode_fixture/bin" HOME="$opencode_fixture/home" XDG_CONFIG_HOME="$opencode_fixture/config" XDG_STATE_HOME="$opencode_fixture/state" \
    OPENCODE_FIXTURE_BIN="$opencode_fixture/bin" OPENCODE_FIXTURE_LOG="$opencode_log" AGE_IDENTITY='fixture-age-key' DOTFILES_SECRETS_FILE="$opencode_fixture/secrets.env.age" \
    bash "$ROOT/install.sh" --apps opencode --yes >/dev/null || failures=$((failures + 1))
if grep -Fq 'anomalyco/tap/opencode' "$opencode_log"; then
    printf 'FAIL: existing OpenCode command triggered a package install\n' >&2
    failures=$((failures + 1))
fi

# Without OpenCode, brew is the preferred available manager and all package
# commands must complete before the setup hook needs jq.
rm -f "$opencode_fixture/bin/opencode" "$opencode_fixture/bin/jq"
: > "$opencode_log"
PATH="$opencode_fixture/bin" HOME="$opencode_fixture/home" XDG_CONFIG_HOME="$opencode_fixture/config" XDG_STATE_HOME="$opencode_fixture/state-missing" \
    OPENCODE_FIXTURE_BIN="$opencode_fixture/bin" OPENCODE_FIXTURE_LOG="$opencode_log" AGE_IDENTITY='fixture-age-key' DOTFILES_SECRETS_FILE="$opencode_fixture/secrets.env.age" \
    bash "$ROOT/install.sh" --apps opencode --yes >/dev/null || failures=$((failures + 1))
grep -Fxq 'package:anomalyco/tap/opencode' "$opencode_log" || { printf 'FAIL: preferred OpenCode package was not installed\n' >&2; failures=$((failures + 1)); }
grep -Fxq 'package:jq' "$opencode_log" || { printf 'FAIL: jq package was not installed\n' >&2; failures=$((failures + 1)); }
[ -x "$opencode_fixture/bin/jq" ] || { printf 'FAIL: jq was unavailable to setup\n' >&2; failures=$((failures + 1)); }
[ "$(grep -n '^package:' "$opencode_log" | tail -n 1 | cut -d: -f1)" -lt "$(grep -n '^setup:jq' "$opencode_log" | head -n 1 | cut -d: -f1)" ] || { printf 'FAIL: setup ran before package installation completed\n' >&2; failures=$((failures + 1)); }
rm -rf "$opencode_fixture"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin" "$tmp/home"
shell_path="$(command -v bash)"
for command in awk bash cat chmod cp date dirname env find grep ln mkdir mv rm sort; do
    command_path="$(type -P "$command" || true)"
    [ -n "$command_path" ] && ln -s "$command_path" "$tmp/bin/$command"
done
[ -e /usr/bin/awk ] && ln -s /usr/bin/awk "$tmp/bin/awk"
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
printf '%s\n' "$*" >> "${PACKAGE_LOG:?}"
if [ "$3" = mise ]; then
    cat > "${MISE_BIN:?}/mise" <<'MISE'
#!/usr/bin/env bash
exit 0
MISE
    chmod +x "${MISE_BIN:?}/mise"
fi
exit 0
EOF
chmod +x "$tmp/bin/apt" "$tmp/bin/dpkg-query" "$tmp/bin/sudo" "$tmp/bin/run0" "$tmp/bin/apt-get"
PATH="$tmp/bin" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state" PACKAGE_LOG="$tmp/packages.log" MISE_BIN="$tmp/bin" bash "$ROOT/install.sh" --apps git --yes >/dev/null || failures=$((failures + 1))
[ -L "$tmp/home/.gitconfig" ] || { printf 'FAIL: package-enabled module was not installed\n' >&2; failures=$((failures + 1)); }
grep -Fxq 'install -y git' "$tmp/packages.log" || { printf 'FAIL: selected package was not installed\n' >&2; failures=$((failures + 1)); }
if grep -Fxq 'install -y mise' "$tmp/packages.log"; then
    printf 'FAIL: mise was bootstrapped for a package with an apt alternative\n' >&2
    failures=$((failures + 1))
fi

rm -f "$tmp/bin/mise"
: > "$tmp/packages-phase.log"
PATH="$tmp/bin" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-phase" PACKAGE_LOG="$tmp/packages-phase.log" MISE_BIN="$tmp/bin" bash "$ROOT/install.sh" --apps mise,git --yes >/dev/null || failures=$((failures + 1))
first=''; second=''
{ IFS= read -r first; IFS= read -r second; } < "$tmp/packages-phase.log"
[ "$first" = 'install -y mise' ] && [ "$second" = 'install -y git' ] || { printf 'FAIL: provider phase did not precede dependent package phase\n' >&2; failures=$((failures + 1)); }

PATH="$tmp/bin" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-mise" PACKAGE_LOG="$tmp/packages.log" MISE_BIN="$tmp/bin" bash "$ROOT/install.sh" --apps mise --yes >/dev/null || failures=$((failures + 1))
grep -Fxq 'install -y mise' "$tmp/packages.log" || { printf 'FAIL: mise module was not installed through apt\n' >&2; failures=$((failures + 1)); }

cat > "$tmp/bin/apt-get" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF
rm -f "$tmp/home/.gitconfig"
if PATH="$tmp/bin" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-failure" bash "$ROOT/install.sh" --apps git --yes >/dev/null 2>&1; then
    printf 'FAIL: package failure succeeded\n' >&2
    failures=$((failures + 1))
fi
[ ! -e "$tmp/home/.gitconfig" ] || { printf 'FAIL: dotfile installed after package failure\n' >&2; failures=$((failures + 1)); }

if PATH="$tmp/bin" HOME="$tmp/home" XDG_STATE_HOME="$tmp/state-atuin" "$shell_path" "$ROOT/install.sh" --apps atuin --yes >/dev/null 2>&1; then
    printf 'FAIL: Atuin package setup unexpectedly succeeded without a package manager\n' >&2
    failures=$((failures + 1))
fi

[ "$failures" -eq 0 ] || exit 1
printf 'package tests passed\n'
