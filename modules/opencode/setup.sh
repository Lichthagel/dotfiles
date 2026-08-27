#!/usr/bin/env bash
set -euo pipefail

required_secrets=(
  OPENROUTER_API_KEY
  AZURE_API_KEY
  AZURE_RESOURCE_NAME
  DIGITALOCEAN_ACCESS_TOKEN
)

for secret_name in "${required_secrets[@]}"; do
  if [[ -z "${!secret_name:-}" ]]; then
    printf 'Missing required OpenCode secret: %s\n' "$secret_name" >&2
    exit 1
  fi
done

config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/opencode"
mkdir -p "$config_dir"
config_file="$config_dir/opencode.json"

if [[ -f "$config_file" ]]; then
  jq empty "$config_file" >/dev/null
fi

credential_tmp="$(mktemp -d "$config_dir/.credentials.XXXXXX")"
config_tmp=''
theme_tmp=''
cleanup() {
  rm -rf "$credential_tmp"
  if [[ -n "$config_tmp" ]]; then
    rm -f "$config_tmp"
  fi
  if [[ -n "$theme_tmp" ]]; then
    rm -rf "$theme_tmp"
  fi
}
trap cleanup EXIT

stage_credential() {
  local name="$1"
  local value="$2"
  printf '%s' "$value" > "$credential_tmp/$name"
  chmod 600 "$credential_tmp/$name"
}

stage_credential openrouter-api-key "$OPENROUTER_API_KEY"
stage_credential azure-api-key "$AZURE_API_KEY"
stage_credential azure-resource-name "$AZURE_RESOURCE_NAME"
stage_credential digitalocean-access-token "$DIGITALOCEAN_ACCESS_TOKEN"

config_tmp="$(mktemp "$config_dir/.opencode.json.XXXXXX")"
jq_args=(
  --arg openrouter "$config_dir/openrouter-api-key" \
  --arg azure_key "$config_dir/azure-api-key" \
  --arg azure_resource "$config_dir/azure-resource-name" \
  --arg digitalocean "$config_dir/digitalocean-access-token" \
  --arg plugin_name 'superpowers@git+https://github.com/obra/superpowers.git'
)
jq_filter=' .provider.openrouter = {options: {apiKey: ("{file:" + $openrouter + "}")}}
   | .provider.azure = {options: {apiKey: ("{file:" + $azure_key + "}"), resourceName: ("{file:" + $azure_resource + "}")}}
   | .provider.digitalocean = {options: {apiKey: ("{file:" + $digitalocean + "}")}}
   | .mcp."ddg-search" = {type: "local", command: ["uvx", "duckduckgo-mcp-server"], environment: {DDG_SAFE_SEARCH: "OFF"}}
   | .plugin = [(.plugin // [])[] | select(. != $plugin_name)] + [$plugin_name]'
if [[ -f "$config_file" ]]; then
  jq "${jq_args[@]}" "$jq_filter" "$config_file" > "$config_tmp"
else
  jq "${jq_args[@]}" "$jq_filter" <<< '{}' > "$config_tmp"
fi
jq empty "$config_tmp" >/dev/null
themes_dir="$config_dir/themes"
theme_api_url="${OPENCODE_THEME_API_URL:-https://api.github.com/repos/catppuccin/opencode/git/trees/main?recursive=1}"
theme_raw_url="${OPENCODE_THEME_RAW_URL:-https://raw.githubusercontent.com/catppuccin/opencode/main}"
theme_tmp="$(mktemp -d "$config_dir/.themes.XXXXXX")"
theme_paths_tmp="$theme_tmp/paths"
printf 'Fetching latest Catppuccin OpenCode theme list...\n' >&2
curl --fail --silent --show-error --location --connect-timeout 10 --max-time 60 "$theme_api_url" \
  | jq -r '.tree[] | select(.type == "blob" and (.path | test("^themes/.+\\.json$"))) | .path' \
  > "$theme_paths_tmp"
[[ -s "$theme_paths_tmp" ]] || { printf 'No Catppuccin OpenCode themes discovered\n' >&2; exit 1; }

theme_count="$(wc -l < "$theme_paths_tmp")"
theme_index=0
while IFS= read -r theme_path; do
  theme_index=$((theme_index + 1))
  printf 'Fetching Catppuccin theme %s/%s: %s\n' "$theme_index" "$theme_count" "$theme_path" >&2
  theme_file="$theme_tmp/$theme_path"
  mkdir -p "$(dirname "$theme_file")"
  curl --fail --silent --show-error --location --connect-timeout 10 --max-time 60 \
    "${theme_raw_url%/}/$theme_path" > "$theme_file"
  jq empty "$theme_file" >/dev/null
done < "$theme_paths_tmp"

# All required work is staged before any managed output is replaced.
for credential in "$credential_tmp"/*; do
  credential_name="${credential##*/}"
  mv -f "$credential" "$config_dir/$credential_name"
done
mv -f "$config_tmp" "$config_file"
config_tmp=''
mkdir -p "$themes_dir"
if [[ -d "$theme_tmp/themes" ]]; then
  find "$theme_tmp/themes" -type f -name '*.json' -print0 | while IFS= read -r -d '' theme_file; do
    relative_path="${theme_file#"$theme_tmp/themes/"}"
    destination="$themes_dir/$relative_path"
    mkdir -p "$(dirname "$destination")"
    mv -f "$theme_file" "$destination"
  done
fi
