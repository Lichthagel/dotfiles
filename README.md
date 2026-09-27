# Dotfiles

Minimal cross-platform dotfiles with interactive selection and deterministic setup.

## Setup

From a local checkout:

```sh
./install.sh
./install.sh --apps git,bash
```

```powershell
.\install.ps1
.\install.ps1 -Apps git,powershell
```

To install OpenCode without the interactive checklist, select it explicitly on
either platform:

```sh
./install.sh --apps opencode
```

```powershell
.\install.ps1 -Apps opencode
```

For this published repository, use the native one-liner. The installer downloads the repository archive when run from a pipe:

```sh
curl -fsSL https://raw.githubusercontent.com/Lichthagel/dotfiles/main/install.sh | sh
```

```powershell
irm https://raw.githubusercontent.com/Lichthagel/dotfiles/main/install.ps1 | iex
```

The installers use the published repository URL by default for piped execution. Set `DOTFILES_REPO_URL` to override it when testing a fork or another repository. The safest automated form is to clone the repository and run the entry point from that checkout.

## Options

- `--apps git,bash` selects modules without prompting on Linux.
- `-Apps git,powershell` is the PowerShell equivalent.
- `--yes` / `-Yes` accepts the package plan without a confirmation prompt.
- `--list` / `-List` lists supported modules.
- `--help` / `-Help` prints usage.

A package is skipped when its command is already in `PATH`, using the logical name from the module declaration, so a tool installed outside the package manager is not reinstalled and needs no available package manager.

## Testing

The repository has isolated Bash and PowerShell installer, package, and secret tests. Run the full local matrix from a checkout with:

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

CI additionally runs ShellCheck and PSScriptAnalyzer. See `CONTRIBUTING.md` for test isolation rules and module coverage expectations.

The initial modules are `git`, `bash`, `powershell`, `atuin`, `oh-my-posh`, and `opencode`. `atuin`, `oh-my-posh`, and `opencode` are opt-in. Existing targets are moved into a timestamped backup directory before installation. Symlinks are preferred; Windows falls back to copying when link creation is unavailable.

### Oh My Posh

Oh My Posh is available as an opt-in cross-platform prompt module. Select `oh-my-posh` interactively or use `--apps bash,oh-my-posh` / `-Apps powershell,oh-my-posh` alongside the corresponding shell module. The module installs the shared prompt configuration and a shell-specific integration drop-in.

### Atuin

Atuin is available as an opt-in package module. Select `atuin` interactively or use `--apps atuin` / `-Apps atuin`. The module installs the Atuin package and, when the encrypted secrets file is present, performs sync login; shell integration remains separate setup behavior.

When `atuin` is selected and `secrets/atuin.env.age` exists, the installer decrypts it temporarily and runs Atuin sync login. It expects `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`; `ATUIN_SYNC_ADDRESS` is optional. Set `AGE_IDENTITIES` to provide an identity-file path or `AGE_IDENTITY` to provide the age private key contents directly.

Shell modules load numbered integration drop-ins from their platform-specific profile directories. Select both modules to enable an integration: `--apps bash,atuin` installs Atuin's Bash drop-in, and `-Apps powershell,atuin` installs its PowerShell drop-in. Selecting only `atuin` performs package and login setup without changing a shell profile. Module selection does not uninstall an existing drop-in.

### OpenCode

OpenCode is an opt-in cross-platform module. Select it interactively or use
`--apps opencode` on Linux or `-Apps opencode` on Windows. If you use the
encrypted secrets flow, create a local plaintext environment file containing
`OPENROUTER_API_KEY`, `AZURE_API_KEY`, `AZURE_RESOURCE_NAME`, and
`DIGITALOCEAN_ACCESS_TOKEN`, then encrypt it as `secrets/opencode.env.age`:

```sh
age -r AGE_RECIPIENT -o secrets/opencode.env.age opencode.env
rm opencode.env
```

The installer decrypts that bundle only during setup when an age identity/key
is available. Without one, the optional provider setup is skipped. OpenCode
stores decrypted values in protected, user-local files in its configuration
directory, and the global `opencode.jsonc` contains `{file:...}` references
rather than secret values. On Windows it is stored at
`~/.config/opencode/opencode.jsonc`.
Never commit the plaintext environment file, an age private key, or decrypted
credential files. Configure an age identity through `AGE_IDENTITIES`,
`AGE_IDENTITY`, or the platform default before selecting the module.

Each setup rerun fetches the latest Catppuccin OpenCode themes from upstream.
The module does not select or add a default theme. OpenCode also requires
`uvx duckduckgo-mcp-server` to be available at runtime for its local MCP
server. The Superpowers plugin is loaded from its configured Git URL:
`superpowers@git+https://github.com/obra/superpowers.git`.

Any selected module with a matching `secrets/<module>.env.age` bundle automatically adds the generic `age` dependency to the package plan. No module manifest entry is needed for the dependency.

Interactive setup uses a checklist. Use Up/Down to move, Space to select or deselect an application, Enter to confirm, or Esc/q to cancel. Each module declares `default=true` or `default=false` in its `module.conf`; defaults are selected when the checklist opens.

Modules can declare manager-specific packages with entries such as `package=git|apt:git` and `package=git|winget:Git.Git`. Linux prefers `apt`, `dnf`, and `pacman`, then falls back to `brew` and `mise`. Windows prefers `winget`, then falls back to `scoop`, `brew`, and `mise`. `run0` is preferred over `sudo` for system managers.

When packages are missing, the package checklist lets you use Up/Down to move, Left/Right to choose a manager per package, Space to include or exclude a package, Enter to continue, and `b` to return to module selection. Esc/q cancels setup. Deselecting a missing package still installs the selected module's dotfiles and prints a warning.

## Adding Modules

Add a module directory, its `module.conf`, source files, and a `module=name` line to `modules/manifest.conf`. Keep mappings platform-qualified and use paths relative to the module directory and user home respectively. Shell integrations should add a numbered drop-in and declare the required shell with `|requires=bash` or `|requires=powershell` instead of duplicating the complete profile.
