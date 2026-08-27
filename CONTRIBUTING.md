# Contributing

## Test Requirements

Run the focused platform test after changing its installer or module declarations. Before opening a change, run the complete local matrix when the tools are available:

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

The tests use isolated temporary homes and fake package-manager commands. They do not install packages on the host. Use `--apps`/`-Apps` and `--yes`/`-Yes` for noninteractive installer checks.

OpenCode coverage must preserve both platform setup declarations and the
opt-in behavior. Its setup fetches the latest upstream Catppuccin themes on
each rerun without adding a default theme, writes provider credentials to
protected user-local files, and keeps secret values out of `opencode.json`.
The configured local MCP prerequisite is `uvx duckduckgo-mcp-server`, and the
Superpowers plugin must continue to use its configured Git URL.

The GitHub Actions workflow also runs ShellCheck for shell files and PSScriptAnalyzer for PowerShell files. Install those tools locally when changing installer logic:

```sh
shellcheck --severity=error install.sh lib/*.sh modules/**/*.sh tests/*.sh
```

```powershell
Install-Module PSScriptAnalyzer -Scope CurrentUser
Invoke-ScriptAnalyzer -Path . -Recurse -Severity Error -EnableExit
```

## Adding A Module

Add the module manifest and source files first, then add tests for its parser declarations and at least one isolated installation path on each supported platform. Cover package selection, setup ordering, mappings, required modules, and backup behavior where applicable.

Keep test-only fake commands and fixtures under `tests/`; never add plaintext credentials, age keys, or decrypted secret files to the repository.

For OpenCode secret tests, use synthetic environment values only. The
encrypted bundle contract is `secrets/opencode.env.age` with
`OPENROUTER_API_KEY`, `AZURE_API_KEY`, `AZURE_RESOURCE_NAME`, and
`DIGITALOCEAN_ACCESS_TOKEN`; do not commit that bundle's plaintext source or
any generated credential files.
