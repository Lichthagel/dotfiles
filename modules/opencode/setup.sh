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
cleanup() {
  rm -rf "$credential_tmp"
  if [[ -n "$config_tmp" ]]; then
    rm -f "$config_tmp"
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
