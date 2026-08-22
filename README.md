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
- `--list` / `-List` lists supported modules.
- `--help` / `-Help` prints usage.

The initial modules are `git`, `bash`, and `powershell`. Existing targets are moved into a timestamped backup directory before installation. Symlinks are preferred; Windows falls back to copying when link creation is unavailable.

Interactive setup uses a checklist. Use Up/Down to move, Space to select or deselect an application, Enter to confirm, or Esc/q to cancel. Each module declares `default=true` or `default=false` in its `module.conf`; defaults are selected when the checklist opens.

## Adding Modules

Add a module directory, its `module.conf`, source files, and a `module=name` line to `modules/manifest.conf`. Keep mappings platform-qualified and use paths relative to the module directory and user home respectively.
