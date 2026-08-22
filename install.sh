#!/usr/bin/env bash
set -u

usage() {
    cat <<'EOF'
Usage: install.sh [--apps name1,name2] [--yes] [--list] [--help]

Install selected dotfiles modules. Without --apps, selection is interactive.
EOF
}

script_dir() {
    cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd
}

ROOT="$(script_dir)"

if [ ! -f "$ROOT/modules/manifest.conf" ]; then
    repo_url="${DOTFILES_REPO_URL:-}"
    [ -n "$repo_url" ] || { printf 'Set DOTFILES_REPO_URL when running install.sh from a pipe.\n' >&2; exit 2; }
    bootstrap_dir="$(mktemp -d)"
    trap 'rm -rf "$bootstrap_dir"' EXIT
    curl -fsSL "$repo_url/archive/refs/heads/main.tar.gz" -o "$bootstrap_dir/repo.tar.gz" || exit 1
    tar -xzf "$bootstrap_dir/repo.tar.gz" -C "$bootstrap_dir" || exit 1
    extracted="$(find "$bootstrap_dir" -mindepth 1 -maxdepth 1 -type d -print -quit)"
    [ -n "$extracted" ] || { printf 'Repository archive did not contain a root directory.\n' >&2; exit 1; }
    exec "$extracted/install.sh" "$@"
fi

BACKUP_ROOT="${XDG_STATE_HOME:-$HOME/.local/state}/dotfiles/backups"
requested_apps=""
apps_provided=0
list_only=0
yes_mode=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --apps)
            [ "$#" -ge 2 ] || { printf 'Missing value for --apps\n' >&2; exit 2; }
            requested_apps="$2"
            apps_provided=1
            shift 2
            ;;
        --list) list_only=1; shift ;;
        --yes) yes_mode=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
done

declare -a MODULES=()
declare -A DESCRIPTIONS=()
declare -A PLATFORMS=()
declare -A DEFAULTS=()
declare -a MAPS=()
declare -a PACKAGES=()
declare -a SETUPS=()
declare -a DEPENDENCIES=()

load_module() {
    local module="$1" config="$ROOT/modules/$1/module.conf" line key value
    [ -f "$config" ] || { printf 'Missing module: %s\n' "$module" >&2; return 1; }
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            name=*) MODULES+=("${line#name=}") ;;
            description=*) DESCRIPTIONS["$module"]="${line#description=}" ;;
            platforms=*) PLATFORMS["$module"]="${line#platforms=}" ;;
            default=*) DEFAULTS["$module"]="${line#default=}" ;;
            map=*) MAPS+=("$module|${line#map=}") ;;
            package=*) PACKAGES+=("$module|${line#package=}") ;;
            setup=*) SETUPS+=("$module|${line#setup=}") ;;
            '') ;;
            \\#*) ;;
            *) printf 'Invalid manifest line in %s: %s\n' "$config" "$line" >&2; return 1 ;;
        esac
    done < "$config"
}

while IFS= read -r module || [ -n "$module" ]; do
    case "$module" in ''|\\#*) continue ;; esac
    module="${module#module=}"
    load_module "$module" || exit 1
done < "$ROOT/modules/manifest.conf"

while IFS= read -r dependency || [ -n "$dependency" ]; do
    case "$dependency" in dependency=*) DEPENDENCIES+=("${dependency#dependency=}") ;; ''|\#*) ;; *) printf 'Invalid dependency line.\n' >&2; exit 1 ;; esac
done < "$ROOT/modules/dependencies.conf"

if [ "$list_only" -eq 1 ]; then
    for module in "${MODULES[@]}"; do
        case ",${PLATFORMS[$module]}," in *,linux,*) printf '%s - %s\n' "$module" "${DESCRIPTIONS[$module]}" ;; esac
    done
    exit 0
fi

is_selected() {
    local needle="$1" item
    IFS=',' read -ra items <<< "$requested_apps"
    for item in "${items[@]}"; do [ "$item" = "$needle" ] && return 0; done
    return 1
}

manager_available() {
    case "$1" in apt|dnf|pacman|brew|mise|scoop|winget) command -v "$1" >/dev/null 2>&1 ;; *) return 1 ;; esac
}

manager_valid() {
    case "$1:$2" in
        linux:apt|linux:dnf|linux:pacman|linux:brew|linux:mise) return 0 ;;
        windows:winget|windows:scoop|windows:brew|windows:mise) return 0 ;;
    esac
    return 1
}

manager_priority=(apt dnf pacman winget brew mise scoop)
package_installed() {
    case "$1" in
        apt) dpkg-query -W -f='${Status}' "$2" 2>/dev/null | grep -q 'install ok installed' ;;
        dnf) rpm -q "$2" >/dev/null 2>&1 ;;
        pacman) pacman -Q "$2" >/dev/null 2>&1 ;;
        brew) brew list --versions "$2" >/dev/null 2>&1 ;;
        mise) mise list 2>/dev/null | awk '{print $1}' | grep -Fxq "$2" ;;
        winget) winget list --id "$2" --exact --accept-source-agreements 2>/dev/null | grep -Fq "$2" ;;
        scoop) scoop list 2>/dev/null | awk '{print $1}' | grep -Fxq "$2" ;;
        *) return 1 ;;
    esac
}

package_plan() {
    PLAN_KEYS=(); PLAN_MODULES=(); PLAN_NAMES=(); PLAN_MANAGERS=(); PLAN_PACKAGE_NAMES=(); PLAN_OPTIONS=(); PLAN_SELECTED=()
    local entry module declaration logical manager name key chosen options candidate selected_module secret_bundle
    for selected_module in "${selected[@]}"; do
        secret_bundle="${DOTFILES_SECRETS_FILE:-$ROOT/secrets/${selected_module}.env.age}"
        if [ -f "$secret_bundle" ]; then
            for dependency in "${DEPENDENCIES[@]}"; do PACKAGES+=("__dependency|$dependency"); done
            break
        fi
    done
    for entry in "${PACKAGES[@]}"; do
        IFS='|' read -r module declaration <<< "$entry"
        [ "$module" = __dependency ] || is_selected "$module" || continue
        IFS='|' read -r logical declaration <<< "$declaration"
        IFS=':' read -r manager name <<< "$declaration"
        key="$module|$logical"
        found=0
        for candidate in "${PLAN_KEYS[@]}"; do [ "$candidate" = "$key" ] && found=1; done
        [ "$found" -eq 1 ] && continue
        options=""
        for priority in "${manager_priority[@]}"; do
            manager_valid linux "$priority" && manager_available "$priority" || continue
            for other in "${PACKAGES[@]}"; do
                IFS='|' read -r other_module other_decl <<< "$other"
                [ "$other_module|${other_decl%%|*}" = "$key" ] || continue
                IFS='|' read -r other_logical other_manager_name <<< "$other_decl"
                IFS=':' read -r other_manager other_name <<< "$other_manager_name"
                [ "$other_manager" = "$priority" ] && options="${options:+$options,}$other_manager:$other_name"
            done
        done
        [ -n "$options" ] || { printf 'No supported package manager is available for %s.\n' "$logical" >&2; return 2; }
        chosen="${options%%,*}"
        manager="${chosen%%:*}"; name="${chosen#*:}"
        package_installed "$manager" "$name" && continue
        PLAN_KEYS+=("$key"); PLAN_MODULES+=("$module"); PLAN_NAMES+=("$logical"); PLAN_MANAGERS+=("$manager"); PLAN_PACKAGE_NAMES+=("$name"); PLAN_OPTIONS+=("$options"); PLAN_SELECTED+=(1)
    done
    [ "${#PLAN_KEYS[@]}" -gt 0 ] || return 0
    if [ "$yes_mode" -eq 1 ]; then return 0; fi
    [ -t 0 ] && [ -t 1 ] || { printf 'Package confirmation requires a terminal. Use --yes for noninteractive setup.\n' >&2; return 2; }
    local index=0 key_input option_index current options_array
    while true; do
        printf '\033[2J\033[HPackage plan (Up/Down move, Left/Right manager, Space toggle, Enter install, b back):\n'
        for ((i=0; i<${#PLAN_NAMES[@]}; i++)); do
            marker=' '; [ "${PLAN_SELECTED[$i]}" -eq 1 ] && marker='x'
            [ "$i" -eq "$index" ] && pointer='>' || pointer=' '
            printf '%s [%s] %s -> %s\n' "$pointer" "$marker" "${PLAN_NAMES[$i]}" "${PLAN_MANAGERS[$i]}"
        done
        IFS= read -rsn1 key_input
        case "$key_input" in
            $'\x1b') IFS=read -rsn2 key_input; case "$key_input" in '[A') [ "$index" -gt 0 ] && index=$((index-1)) ;; '[B') [ "$index" -lt $((${#PLAN_NAMES[@]}-1)) ] && index=$((index+1)) ;; '[C'|'[D') IFS=',' read -ra options_array <<< "${PLAN_OPTIONS[$index]}"; option_index=0; for i in "${!options_array[@]}"; do [ "${options_array[$i]%%:*}" = "${PLAN_MANAGERS[$index]}" ] && option_index=$i; done; [ "$key_input" = '[C' ] && option_index=$(( (option_index + 1) % ${#options_array[@]} )) || option_index=$(( (option_index - 1 + ${#options_array[@]}) % ${#options_array[@]} )); PLAN_MANAGERS[$index]="${options_array[$option_index]%%:*}"; PLAN_PACKAGE_NAMES[$index]="${options_array[$option_index]#*:}" ;; esac ;;
            ' ') [ "${PLAN_SELECTED[$index]}" -eq 1 ] && PLAN_SELECTED[$index]=0 || PLAN_SELECTED[$index]=1 ;;
            b) return 3 ;;
            q) return 2 ;;
            '') break ;;
        esac
    done
    printf '\033[2J\033[H'
    return 0
}

install_package() {
    local manager="$1" package="$2" prefix=()
    case "$manager" in
        apt) command -v run0 >/dev/null 2>&1 && prefix=(run0) || prefix=(sudo); "${prefix[@]}" apt-get install -y "$package" ;;
        dnf) command -v run0 >/dev/null 2>&1 && prefix=(run0) || prefix=(sudo); "${prefix[@]}" dnf install -y "$package" ;;
        pacman) command -v run0 >/dev/null 2>&1 && prefix=(run0) || prefix=(sudo); "${prefix[@]}" pacman -S --needed --noconfirm "$package" ;;
        brew) brew install "$package" ;;
        mise) mise use --global "$package" ;;
        scoop) scoop install "$package" ;;
        winget) winget install --id "$package" --exact --accept-source-agreements --accept-package-agreements ;;
        *) printf 'Unsupported package manager: %s\n' "$manager" >&2; return 2 ;;
    esac
}

run_module_setup() {
    local entry module declaration platform setup_path secret_source secret_keys key
    source "$ROOT/lib/secrets.sh"
    for entry in "${SETUPS[@]}"; do
        IFS='|' read -r module declaration <<< "$entry"
        is_selected "$module" || continue
        IFS=':' read -r platform setup_path <<< "$declaration"
        [ "$platform" = linux ] || continue
        secret_source="$ROOT/secrets/${module}.env.age"
        secret_keys=''
        if [ -n "${DOTFILES_SECRETS_FILE:-}" ] || [ -f "$secret_source" ]; then
            DOTFILES_SECRET_FILE="${DOTFILES_SECRETS_FILE:-$secret_source}"
            DOTFILES_SECRET_KEYS=''
            dotfiles_decrypt_env || return 1
        else
            DOTFILES_SECRET_FILE=''
            DOTFILES_SECRET_KEYS=''
        fi
        DOTFILES_ROOT="$ROOT" DOTFILES_SECRET_FILE="$DOTFILES_SECRET_FILE" DOTFILES_SECRET_KEYS="$DOTFILES_SECRET_KEYS" \
            "$ROOT/modules/$setup_path" || { dotfiles_cleanup_secrets; return 1; }
        dotfiles_cleanup_secrets
    done
}

interactive_select() {
    local -a available_modules=() selected_modules=()
    local index=0 key sequence
    selection_cancelled=0
    for module in "${MODULES[@]}"; do
        case ",${PLATFORMS[$module]}," in
            *,linux,*)
                available_modules+=("$module")
                [ "${DEFAULTS[$module]:-false}" = true ] && selected_modules+=("$module") || selected_modules+=("")
                ;;
        esac
    done
    [ -t 0 ] && [ -t 1 ] || { printf 'Interactive selection requires a terminal. Use --apps for noninteractive setup.\n' >&2; return 2; }
    printf 'Use Up/Down to move, Space to toggle, Enter to confirm.\n'
    while true; do
        printf '\033[2J\033[H'
        printf 'Select applications:\n'
        for ((i = 0; i < ${#available_modules[@]}; i++)); do
            marker=' '
            [ -n "${selected_modules[$i]}" ] && marker='x'
            [ "$i" -eq "$index" ] && pointer='>' || pointer=' '
            printf '%s [%s] %s - %s\n' "$pointer" "$marker" "${available_modules[$i]}" "${DESCRIPTIONS[${available_modules[$i]}]}"
        done
        IFS= read -rsn1 key
        case "$key" in
            $'\x1b')
                if IFS= read -rsn1 -t 0.05 sequence; then
                    if [ "$sequence" = '[' ] && IFS= read -rsn1 sequence; then
                        case "$sequence" in
                            A) [ "$index" -gt 0 ] && index=$((index - 1)) ;;
                            B) [ "$index" -lt $((${#available_modules[@]} - 1)) ] && index=$((index + 1)) ;;
                        esac
                    else
                        selection_cancelled=1
                        break
                    fi
                else
                    selection_cancelled=1
                    break
                fi
                ;;
            q) selection_cancelled=1; break ;;
            ' ') [ -n "${selected_modules[$index]}" ] && selected_modules[$index]='' || selected_modules[$index]="${available_modules[$index]}" ;;
            '') break ;;
        esac
    done
    requested_apps=""
    for module in "${selected_modules[@]}"; do
        [ -n "$module" ] && requested_apps="${requested_apps:+$requested_apps,}$module"
    done
}

if [ "$apps_provided" -eq 0 ]; then
    interactive_select || exit $?
    [ "$selection_cancelled" -eq 0 ] || { printf 'Selection cancelled.\n'; exit 0; }
fi

if [ -z "$requested_apps" ]; then
    printf 'No applications selected.\n'
    exit 0
fi

while true; do
    IFS=',' read -ra selected <<< "$requested_apps"
    for module in "${selected[@]}"; do
        is_known=0
        for known in "${MODULES[@]}"; do [ "$known" = "$module" ] && is_known=1; done
        [ "$is_known" -eq 1 ] || { printf 'Unknown application: %s\n' "$module" >&2; exit 2; }
        case ",${PLATFORMS[$module]}," in *,linux,*) ;; *) printf 'Application not supported on Linux: %s\n' "$module" >&2; exit 2 ;; esac
    done
    package_plan
    package_status=$?
    [ "$package_status" -eq 3 ] && [ "$apps_provided" -eq 0 ] && { interactive_select || exit $?; [ "$selection_cancelled" -eq 0 ] || { printf 'Selection cancelled.\n'; exit 0; }; [ -n "$requested_apps" ] || { printf 'No applications selected.\n'; exit 0; }; continue; }
    [ "$package_status" -eq 0 ] || exit "$package_status"
    break
done

if [ "${#PLAN_NAMES[@]}" -gt 0 ] && [ "$yes_mode" -eq 0 ]; then
    printf 'Install these packages? [y/N]: '
    IFS= read -r package_confirmation
    case "$package_confirmation" in
        y|Y|yes|YES) ;;
        *) printf 'Package installation declined.\n'; exit 0 ;;
    esac
fi

for i in "${!PLAN_NAMES[@]}"; do
    if [ "${PLAN_SELECTED[$i]}" -eq 1 ]; then
        install_package "${PLAN_MANAGERS[$i]}" "${PLAN_PACKAGE_NAMES[$i]}" || { printf 'Package installation failed: %s\n' "${PLAN_NAMES[$i]}" >&2; exit 1; }
    else
        printf 'Skipped package %s for module %s; dotfiles were still installed.\n' "${PLAN_NAMES[$i]}" "${PLAN_MODULES[$i]}"
    fi
done

run_module_setup || exit 1

timestamp="$(date +%Y%m%d-%H%M%S)"
failures=0
for entry in "${MAPS[@]}"; do
    IFS='|' read -r module mapping <<< "$entry"
    is_selected "$module" || continue
    IFS='|' read -r platform_source target_relative <<< "$mapping"
    case "$platform_source" in linux:*) source_relative="${platform_source#linux:}" ;; *) continue ;; esac
    source="$ROOT/modules/$source_relative"
    target="$HOME/$target_relative"
    [ -e "$source" ] || { printf 'Missing source: %s\n' "$source" >&2; failures=$((failures + 1)); continue; }
    mkdir -p "$(dirname -- "$target")"
    if [ -e "$target" ] || [ -L "$target" ]; then
        backup="$BACKUP_ROOT/$timestamp/$target_relative"
        mkdir -p "$(dirname -- "$backup")"
        mv -- "$target" "$backup" || { printf 'failed backup: %s\n' "$target" >&2; failures=$((failures + 1)); continue; }
        printf 'backed up: %s -> %s\n' "$target" "$backup"
    fi
    if ln -s "$source" "$target" 2>/dev/null; then
        printf 'linked: %s\n' "$target"
    elif cp -R -- "$source" "$target"; then
        printf 'copied: %s\n' "$target"
    else
        printf 'failed: %s\n' "$target" >&2
        failures=$((failures + 1))
    fi
done
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
