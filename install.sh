#!/usr/bin/env bash
set -u

usage() {
    cat <<'EOF'
Usage: install.sh [--apps name1,name2] [--list] [--help]

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

while [ "$#" -gt 0 ]; do
    case "$1" in
        --apps)
            [ "$#" -ge 2 ] || { printf 'Missing value for --apps\n' >&2; exit 2; }
            requested_apps="$2"
            apps_provided=1
            shift 2
            ;;
        --list) list_only=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
done

declare -a MODULES=()
declare -A DESCRIPTIONS=()
declare -A PLATFORMS=()
declare -A DEFAULTS=()
declare -a MAPS=()

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

IFS=',' read -ra selected <<< "$requested_apps"
for module in "${selected[@]}"; do
    is_known=0
    for known in "${MODULES[@]}"; do [ "$known" = "$module" ] && is_known=1; done
    [ "$is_known" -eq 1 ] || { printf 'Unknown application: %s\n' "$module" >&2; exit 2; }
    case ",${PLATFORMS[$module]}," in *,linux,*) ;; *) printf 'Application not supported on Linux: %s\n' "$module" >&2; exit 2 ;; esac
done

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
