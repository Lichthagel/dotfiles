# OpenCode Module Design

## Goal

Add an opt-in cross-platform `opencode` module that installs OpenCode when it is unavailable and configures OpenRouter, Azure OpenAI, DigitalOcean, Catppuccin themes, the DuckDuckGo MCP server, and the Superpowers plugin without storing credentials or theme files in this repository.

## Scope

The module will support Linux and Windows through the existing restricted module manifest and installer contracts. It will not define model aliases, a default model, a default theme, or provider model maps.

The module will:

- Install OpenCode only when the selected platform does not already provide it.
- Configure provider authentication from an encrypted `secrets/opencode.env.age` bundle.
- Merge only module-owned keys into the existing global OpenCode configuration.
- Fetch the latest Catppuccin OpenCode themes from the upstream `main` branch during setup.
- Configure the requested DuckDuckGo MCP server and Superpowers plugin.

The module will not:

- Store Catppuccin theme files in the repository.
- Store plaintext credentials, decrypted secret bundles, or API keys in the repository.
- Add OpenCode-specific behavior to `install.sh` or `install.ps1`.
- Configure model aliases or select a default theme.

## Secret Flow

The encrypted bundle will be `secrets/opencode.env.age`. It will contain these required values:

- `OPENROUTER_API_KEY`
- `AZURE_API_KEY`
- `AZURE_RESOURCE_NAME`
- `DIGITALOCEAN_ACCESS_TOKEN`

The existing installer will decrypt the bundle only for the module setup hook. The setup hook will validate all required values, write one provider credential value per local file under the user's OpenCode configuration directory, and remove temporary decrypted material before returning.

The generated OpenCode configuration will reference those files with OpenCode `{file:...}` substitutions. The setup-time environment is therefore not expected to persist after the installer exits. Credential files will receive mode `600` on Linux and user-only access control on Windows.

If the bundle is absent or a required value is missing, provider configuration will fail clearly before changing managed credentials or configuration. The repository will document the expected secret names without including values.

## OpenCode Installation

The module will declare platform-qualified package entries using existing package-manager support:

- Linux Homebrew: `anomalyco/tap/opencode`
- Linux mise: `github:anomalyco/opencode`
- Windows Scoop: `opencode`
- Windows mise: `github:anomalyco/opencode`
- Windows Homebrew: `anomalyco/tap/opencode`

The installer’s existing package detection will skip the package when the relevant manager reports OpenCode already installed. If no supported available manager can install OpenCode, setup will fail with the existing package-plan error.

## Configuration Merge

The setup hook will locate the global OpenCode configuration at the platform’s standard user configuration path, create the parent directory when needed, and merge the module-owned keys into it. Existing unrelated keys will be preserved.

Managed keys are:

- `provider.openrouter`
- `provider.azure`
- `provider.digitalocean`
- `mcp.ddg-search`
- `plugin`

Provider entries will contain authentication/configuration references only. They will not set `model`, `small_model`, aliases, model maps, whitelists, or blacklists.

The managed MCP entry will be:

```json
{
  "ddg-search": {
    "type": "local",
    "command": ["uvx", "duckduckgo-mcp-server"],
    "environment": {
      "DDG_SAFE_SEARCH": "OFF"
    }
  }
}
```

The managed plugin list will include:

```text
superpowers@git+https://github.com/obra/superpowers.git
```

The merge must be idempotent. Re-running setup replaces the module-owned values without duplicating plugin entries or damaging unrelated configuration. Invalid existing JSON will fail safely without overwriting the original file.

Because Linux does not guarantee a JSON processor, the implementation will provide the required merge utility through the module's supported package declarations or dependency declarations. The selected approach must work in isolated Linux and PowerShell tests without requiring Node or Python.

## Catppuccin Themes

Theme files will not be vendored. Each setup run will fetch the current generated theme files from `catppuccin/opencode` on the upstream `main` branch. The setup will discover all current flavor/accent files rather than hard-code only the currently known combinations.

The setup will install all downloaded theme files into the user's OpenCode themes directory while preserving unrelated local themes. It will not write a theme selection to `tui.json` or `opencode.json`.

Network failures, malformed downloads, or incomplete theme sets will fail setup clearly rather than silently producing a partial installation. The setup must use temporary download locations and clean them up on success or failure.

## Setup Boundaries

The module will have platform-specific setup hooks, following the existing `setup=linux:` and `setup=windows:` declarations. Those hooks own:

- Secret validation and credential-file writes
- OpenCode configuration merging
- Upstream theme discovery and downloads
- Platform-specific permissions

The installers continue to own module discovery, package planning, secret decryption lifecycle, backups for ordinary mappings, and setup ordering. No installer branch will mention OpenCode, Catppuccin, DuckDuckGo, or Superpowers.

## Failure Handling

- Missing encrypted secrets: fail provider setup with a clear missing-bundle message.
- Missing required secret: fail before managed files are replaced.
- Existing invalid JSON: preserve the original and fail.
- Theme download failure or incomplete discovery: fail and clean temporary files.
- Credential permission failure: fail rather than leaving credentials broadly readable.
- Existing unrelated OpenCode settings: preserve them.

## Testing

Add isolated Bash and PowerShell coverage for:

- Manifest registration, metadata, platform declarations, and opt-in default.
- OpenCode package declarations and already-installed package skipping.
- Required secret names and absence of plaintext credentials in tracked files.
- Credential-file creation from a fake encrypted bundle.
- Credential-file permissions or ACL behavior where the platform test supports it.
- Preservation of unrelated JSON keys during merge.
- Replacement and idempotency of managed provider, MCP, and plugin keys.
- Failure on invalid existing JSON and incomplete secrets.
- Theme discovery/download using fake network or fixture commands, including all discovered files and no default theme.
- Exact DuckDuckGo MCP command/environment and Superpowers plugin value.

The normal focused and full repository test commands remain the verification gate:

```sh
bash -n install.sh
bash tests/test-install.sh
bash tests/test-packages.sh
bash tests/test-secrets.sh
```

```powershell
pwsh -NoProfile -File .\tests\test-install.ps1
pwsh -NoProfile -File .\tests\test-packages.ps1
```
