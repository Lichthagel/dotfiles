#!/usr/bin/env bash

dotfiles_decrypt_env() {
    local identity="${AGE_IDENTITIES:-$HOME/.config/age/keys.txt}" identity_input
    [ -n "${DOTFILES_SECRET_FILE:-}" ] && [ -f "$DOTFILES_SECRET_FILE" ] || { printf 'Encrypted secrets file not found.\n' >&2; return 1; }
    command -v age >/dev/null 2>&1 || { printf 'age is required for secrets setup.\n' >&2; return 1; }
    DOTFILES_SECRET_TEMP=''
    if [ -n "${AGE_IDENTITY:-}" ]; then
        DOTFILES_SECRET_TEMP="$(mktemp -d)"
        chmod 700 "$DOTFILES_SECRET_TEMP"
        identity="$DOTFILES_SECRET_TEMP/identity.txt"
        printf '%s\n' "$AGE_IDENTITY" > "$identity"
        chmod 600 "$identity"
    elif [ ! -f "$identity" ]; then
        if [ -t 0 ] && [ -t 1 ]; then
            printf 'Age identity file path or key (default not found): '
            IFS= read -r identity_input
        elif [ -t 1 ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
            exec 3<>/dev/tty
            printf 'Age identity file path or key (default not found): ' >&3
            IFS= read -u 3 -r identity_input
            exec 3>&-
        else
            printf 'Age identity file not found. Set AGE_IDENTITIES for noninteractive setup.\n' >&2
            return 1
        fi
        if [ -n "$identity_input" ]; then
            if [[ "$identity_input" == AGE-SECRET-KEY-* ]]; then
                DOTFILES_SECRET_TEMP="$(mktemp -d)"
                chmod 700 "$DOTFILES_SECRET_TEMP"
                identity="$DOTFILES_SECRET_TEMP/identity.txt"
                printf '%s\n' "$identity_input" > "$identity"
                chmod 600 "$identity"
            else
                identity="$identity_input"
            fi
        elif [ "${DOTFILES_SECRETS_OPTIONAL:-}" = 1 ]; then
            return 3
        fi
    fi
    [ -f "$identity" ] || { printf 'Age identity file not found.\n' >&2; return 1; }
    [ -n "$DOTFILES_SECRET_TEMP" ] || DOTFILES_SECRET_TEMP="$(mktemp -d)"
    DOTFILES_SECRET_KEYS_LOADED=''
    chmod 700 "$DOTFILES_SECRET_TEMP"
    DOTFILES_SECRET_PLAINTEXT="$DOTFILES_SECRET_TEMP/secrets.env"
    if ! age --decrypt -i "$identity" "$DOTFILES_SECRET_FILE" > "$DOTFILES_SECRET_PLAINTEXT"; then
        dotfiles_cleanup_secrets
        printf 'Unable to decrypt secrets.\n' >&2
        return 1
    fi
    chmod 600 "$DOTFILES_SECRET_PLAINTEXT"
    local key line value
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in ''|\#*) continue ;; esac
        case "$line" in *=*) ;; *) dotfiles_cleanup_secrets; printf 'Invalid secrets format.\n' >&2; return 1 ;; esac
        key="${line%%=*}"
        [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { dotfiles_cleanup_secrets; printf 'Invalid secret name.\n' >&2; return 1; }
        value="${line#*=}"
        export "$key=$value"
        DOTFILES_SECRET_KEYS_LOADED="${DOTFILES_SECRET_KEYS_LOADED:+$DOTFILES_SECRET_KEYS_LOADED,}$key"
    done < "$DOTFILES_SECRET_PLAINTEXT"
}

dotfiles_secret_value() {
    local wanted="$1" line key value
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in ''|\#*) continue ;; esac
        case "$line" in *=*) ;; *) continue ;; esac
        key="${line%%=*}"
        [ "$key" = "$wanted" ] || continue
        value="${line#*=}"
        [ -n "$value" ] || return 1
        printf '%s' "$value"
        return 0
    done < "$DOTFILES_SECRET_PLAINTEXT"
    return 1
}

dotfiles_cleanup_secrets() {
    [ -n "${DOTFILES_SECRET_TEMP:-}" ] && rm -rf "$DOTFILES_SECRET_TEMP"
    local key
    IFS=',' read -ra loaded_keys <<< "${DOTFILES_SECRET_KEYS_LOADED:-}"
    for key in "${loaded_keys[@]}"; do [ -n "$key" ] && unset "$key"; done
}
