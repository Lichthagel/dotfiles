#!/usr/bin/env bash
set -u
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
failures=0

manifest="$ROOT/modules/manifest.conf"
module="$ROOT/modules/opencode/module.conf"

grep -Fq 'module=opencode' "$manifest" || failures=$((failures + 1))
grep -Fq 'name=opencode' "$module" || failures=$((failures + 1))
grep -Fq 'platforms=linux,windows' "$module" || failures=$((failures + 1))
grep -Fq 'default=false' "$module" || failures=$((failures + 1))
grep -Fq 'setup=linux:opencode/setup.sh' "$module" || failures=$((failures + 1))
grep -Fq 'setup=windows:opencode/setup.ps1' "$module" || failures=$((failures + 1))

grep -Fq 'package=opencode|brew:anomalyco/tap/opencode' "$module" || failures=$((failures + 1))
grep -Fq 'package=opencode|mise:github:anomalyco/opencode' "$module" || failures=$((failures + 1))
grep -Fq 'package=opencode|scoop:opencode' "$module" || failures=$((failures + 1))

grep -Fq 'package=jq|apt:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|dnf:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|pacman:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|brew:jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|winget:jqlang.jq' "$module" || failures=$((failures + 1))
grep -Fq 'package=jq|scoop:jq' "$module" || failures=$((failures + 1))

setup="$ROOT/modules/opencode/setup.sh"
[ -x "$setup" ] || failures=$((failures + 1))
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
config_root="$fixture/config"
config_dir="$config_root/opencode"
mkdir -p "$config_dir"
config="$config_dir/opencode.json"
cat > "$config" <<'JSON'
{"unrelated":{"keep":true},"plugin":["local-plugin","superpowers@git+https://github.com/obra/superpowers.git","superpowers@git+https://github.com/obra/superpowers.git"]}
JSON

export HOME="$fixture/home"
export XDG_CONFIG_HOME="$config_root"
export OPENROUTER_API_KEY='openrouter-test-secret'
export AZURE_API_KEY='azure-test-secret'
export AZURE_RESOURCE_NAME='azure-resource-test'
export DIGITALOCEAN_ACCESS_TOKEN='digitalocean-test-secret'

if ! bash "$setup"; then
  failures=$((failures + 1))
else
  jq -e '.unrelated.keep == true' "$config" >/dev/null || failures=$((failures + 1))
  jq -e --arg path "$config_dir/openrouter-api-key" '.provider.openrouter.options.apiKey == ("{file:" + $path + "}")' "$config" >/dev/null || failures=$((failures + 1))
  jq -e --arg path "$config_dir/azure-api-key" --arg resource "$config_dir/azure-resource-name" '.provider.azure.options.apiKey == ("{file:" + $path + "}") and .provider.azure.options.resourceName == ("{file:" + $resource + "}")' "$config" >/dev/null || failures=$((failures + 1))
  jq -e --arg path "$config_dir/digitalocean-access-token" '.provider.digitalocean.options.apiKey == ("{file:" + $path + "}")' "$config" >/dev/null || failures=$((failures + 1))
  for secret_name in OPENROUTER_API_KEY AZURE_API_KEY AZURE_RESOURCE_NAME DIGITALOCEAN_ACCESS_TOKEN; do
    jq -e --arg secret "${!secret_name}" '([.. | strings] | index($secret)) == null' "$config" >/dev/null || failures=$((failures + 1))
  done
  jq -e '.mcp["ddg-search"].type == "local" and .mcp["ddg-search"].command == ["uvx", "duckduckgo-mcp-server"] and .mcp["ddg-search"].environment.DDG_SAFE_SEARCH == "OFF"' "$config" >/dev/null || failures=$((failures + 1))
  [ "$(jq '[.plugin[] | select(. == "superpowers@git+https://github.com/obra/superpowers.git")] | length' "$config")" -eq 1 ] || failures=$((failures + 1))
  [ "$(cat "$config_dir/openrouter-api-key")" = "$OPENROUTER_API_KEY" ] || failures=$((failures + 1))
  [ "$(cat "$config_dir/azure-api-key")" = "$AZURE_API_KEY" ] || failures=$((failures + 1))
  [ "$(cat "$config_dir/azure-resource-name")" = "$AZURE_RESOURCE_NAME" ] || failures=$((failures + 1))
  [ "$(cat "$config_dir/digitalocean-access-token")" = "$DIGITALOCEAN_ACCESS_TOKEN" ] || failures=$((failures + 1))
  for credential in openrouter-api-key azure-api-key azure-resource-name digitalocean-access-token; do
    [ "$(stat -c '%a' "$config_dir/$credential")" = 600 ] || failures=$((failures + 1))
  done
  bash "$setup" || failures=$((failures + 1))
  [ "$(jq '[.plugin[] | select(. == "superpowers@git+https://github.com/obra/superpowers.git")] | length' "$config")" -eq 1 ] || failures=$((failures + 1))

  config_before="$(cat "$config")"
  unset AZURE_API_KEY
  if bash "$setup"; then
    failures=$((failures + 1))
  fi
  [ "$(cat "$config")" = "$config_before" ] || failures=$((failures + 1))
  export AZURE_API_KEY='azure-test-secret'
fi

printf '{invalid' > "$config"
invalid_before="$(cat "$config")"
if bash "$setup"; then
  failures=$((failures + 1))
fi
[ "$(cat "$config")" = "$invalid_before" ] || failures=$((failures + 1))

[ "$failures" -eq 0 ] || exit 1
printf 'OpenCode Bash assertions passed\n'
