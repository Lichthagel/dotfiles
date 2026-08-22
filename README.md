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

For a published repository, use the native one-liner after replacing the URL with its canonical GitHub URL. The installer downloads the repository archive when run from a pipe:

```sh
curl -fsSL https://raw.githubusercontent.com/OWNER/dotfiles/main/install.sh | DOTFILES_REPO_URL=https://github.com/OWNER/dotfiles sh
```

```powershell
& { $env:DOTFILES_REPO_URL = 'https://github.com/OWNER/dotfiles'; irm https://raw.githubusercontent.com/OWNER/dotfiles/main/install.ps1 | iex }
```

The `DOTFILES_REPO_URL` value is required for piped execution because the script itself is hosted separately from the module files. The safest automated form is to clone the repository and run the entry point from that checkout.

## Options

- `--apps git,bash` selects modules without prompting on Linux.
- `-Apps git,powershell` is the PowerShell equivalent.
- `--yes` / `-Yes` accepts the package plan without a confirmation prompt.
- `--list` / `-List` lists supported modules.
- `--help` / `-Help` prints usage.

The initial modules are `git`, `bash`, and `powershell`. Existing targets are moved into a timestamped backup directory before installation. Symlinks are preferred; Windows falls back to copying when link creation is unavailable.

### Atuin

Atuin is available as an opt-in package module. Select `atuin` interactively or use `--apps atuin` / `-Apps atuin`. The module installs the Atuin package and, when the encrypted secrets file is present, performs sync login; shell integration remains separate setup behavior.

When `atuin` is selected and `secrets/atuin.env.age` exists, the installer decrypts it temporarily and runs Atuin sync login. It expects `ATUIN_USERNAME`, `ATUIN_PASSWORD`, and `ATUIN_KEY`; `ATUIN_SYNC_ADDRESS` is optional. Set `AGE_IDENTITIES` to provide an identity-file path or `AGE_IDENTITY` to provide the age private key contents directly.

Any selected module with a matching `secrets/<module>.env.age` bundle automatically adds the generic `age` dependency to the package plan. No module manifest entry is needed for the dependency.

Interactive setup uses a checklist. Use Up/Down to move, Space to select or deselect an application, Enter to confirm, or Esc/q to cancel. Each module declares `default=true` or `default=false` in its `module.conf`; defaults are selected when the checklist opens.

Modules can declare manager-specific packages with entries such as `package=git|apt:git` and `package=git|winget:Git.Git`. Linux prefers `apt`, `dnf`, and `pacman`, then falls back to `brew` and `mise`. Windows prefers `winget`, then falls back to `scoop`, `brew`, and `mise`. `run0` is preferred over `sudo` for system managers.

When packages are missing, the package checklist lets you use Up/Down to move, Left/Right to choose a manager per package, Space to include or exclude a package, Enter to continue, and `b` to return to module selection. Esc/q cancels setup. Deselecting a missing package still installs the selected module's dotfiles and prints a warning.

## Adding Modules

Add a module directory, its `module.conf`, source files, and a `module=name` line to `modules/manifest.conf`. Keep mappings platform-qualified and use paths relative to the module directory and user home respectively.
