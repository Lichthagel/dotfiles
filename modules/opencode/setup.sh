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

write_credential() {
  local name="$1"
  local value="$2"
  local target="$config_dir/$name"
  printf '%s' "$value" > "$credential_tmp/$name"
  chmod 600 "$credential_tmp/$name"
  mv -f "$credential_tmp/$name" "$target"
}

write_credential openrouter-api-key "$OPENROUTER_API_KEY"
write_credential azure-api-key "$AZURE_API_KEY"
write_credential azure-resource-name "$AZURE_RESOURCE_NAME"
write_credential digitalocean-access-token "$DIGITALOCEAN_ACCESS_TOKEN"

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
mv -f "$config_tmp" "$config_file"
config_tmp=''

themes_dir="$config_dir/themes"
theme_api_url="${OPENCODE_THEME_API_URL:-https://api.github.com/repos/catppuccin/opencode/git/trees/main?recursive=1}"
theme_raw_url="${OPENCODE_THEME_RAW_URL:-https://raw.githubusercontent.com/catppuccin/opencode/main}"
theme_tmp="$(mktemp -d "$config_dir/.themes.XXXXXX")"
theme_paths_tmp="$theme_tmp/paths"
curl --fail --silent --show-error --location "$theme_api_url" \
  | jq -r '.tree[] | select(.type == "blob" and (.path | startswith("themes/") and endswith(".json"))) | .path' \
  > "$theme_paths_tmp"
[[ -s "$theme_paths_tmp" ]] || { printf 'No Catppuccin OpenCode themes discovered\n' >&2; exit 1; }

while IFS= read -r theme_path; do
  theme_name="${theme_path##*/}"
  curl --fail --silent --show-error --location \
    "${theme_raw_url%/}/$theme_path" > "$theme_tmp/$theme_name"
  jq empty "$theme_tmp/$theme_name" >/dev/null
done < "$theme_paths_tmp"

mkdir -p "$themes_dir"
find "$themes_dir" -maxdepth 1 -type f -name 'catppuccin-*.json' -delete
find "$theme_tmp" -maxdepth 1 -type f -name '*.json' -exec mv -f {} "$themes_dir/" \;
