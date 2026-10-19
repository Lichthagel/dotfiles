# Modules

The installer modules are listed below. The explicit catalog in
[`src/registry.rs`](../src/registry.rs) is authoritative; each module's
configuration and setup live in [`src/modules/`](../src/modules/).

| Module | Platforms | Selected by default | Documentation |
| --- | --- | --- | --- |
| `git` | Linux, Windows | Yes | [Git](modules/git.md) |
| `bash` | Linux | Yes | [Bash](modules/bash.md) |
| `zsh` | Linux | Yes | [Zsh](modules/zsh.md) |
| `powershell` | Windows | Yes | [PowerShell](modules/powershell.md) |
| `mise` | Linux, Windows | Yes | [mise](modules/mise.md) |
| `brew` | Linux | No | [Homebrew](modules/brew.md) |
| `scoop` | Windows | No | [Scoop](modules/scoop.md) |
| `atuin` | Linux, Windows | No | [Atuin](modules/atuin.md) |
| `oh-my-posh` | Linux, Windows | No | [Oh My Posh](modules/oh-my-posh.md) |
| `opencode` | Linux, Windows | No | [OpenCode](modules/opencode.md) |

See [Installation](installation.md) for module selection, package planning,
shell integration requirements, and applied-module state.
