# Modules

The installer supports the modules below. Each module's `SPEC`, embedded
configuration/profile assets, and any module-specific setup live in
[`src/modules/`](../src/modules/); the explicit catalog in
[`src/registry.rs`](../src/registry.rs) remains authoritative. Atuin and
OpenCode also embed their encrypted secret bundles beside their module
specifications; the source bundles remain under `secrets/`.
The interactive checklist preselects the defaults shown below. Press Enter to
approve that selection; use Space to change it. Previously applied state does
not affect the defaults. Provider modules must be selected explicitly when
to install or configure a manager; the installer never adds modules to the
selection implicitly. Already available managers can be used without selecting
their provider module.

| Module | Platforms | Selected by default | Purpose |
| --- | --- | --- | --- |
| `git` | Linux, Windows | Yes | Installs Git and writes `~/.gitconfig`. |
| `bash` | Linux | Yes | Updates a managed section in `~/.bashrc`, which loads Bash integration drop-ins. |
| `zsh` | Linux | Yes | Updates a managed section in `~/.zshrc`, which loads Zsh integration drop-ins. |
| `powershell` | Windows | Yes | Updates a managed section in the active PowerShell profile, using `$PROFILE` or the Windows Documents known folder, and loads PowerShell integration drop-ins. |
| `mise` | Linux, Windows | Yes | Installs the mise tool version manager and provides optional shell activation. On Debian/Ubuntu and Fedora/RHEL, it enables mise's APT or RPM repository when the package is not already available. |
| `brew` | Linux | No | Sets up Homebrew as a package manager and provides optional Bash and Zsh integration. |
| `scoop` | Windows | No | Sets up Scoop as a package manager and provides optional PowerShell integration. |
| `atuin` | Linux, Windows | No | Installs Atuin shell history, provides optional shell integration, and configures sync login when credentials are available. |
| `oh-my-posh` | Linux, Windows | No | Installs Oh My Posh, writes its configuration to `~/.config/oh-my-posh/config.json` on Linux or beside the PowerShell profile on Windows, and provides optional shell integration. |
| `opencode` | Linux, Windows | No | Installs OpenCode, writes a v2 config referencing the [JSON Schema](https://opencode.ai/config.json), fetches themes, and optionally configures provider credentials. |

## Selecting Modules

Use comma-separated module IDs to skip the interactive checklist:

```sh
cargo run -- --modules git,bash,mise --yes
```

```powershell
cargo run -- --modules git,powershell,mise --yes
```

`--apps` is a compatibility alias for `--modules`. `--yes` accepts the package
plan and installation order without prompting; it does not authorize additional
modules. Unattended setup still requires explicit module selection. Use `--list`
to list modules supported by the current binary.

Each module declares packages as a `Package` with a stable `id`, an optional
`command`, and nested `PackageSource` alternatives (`manager` and package `name`).
The ID groups sources into one installation requirement and labels package
selection and diagnostics; it does not determine the executable name. For
example, Git declares ID `git`, command `Some("git")`, and a Winget source named
`Git.Git`. Package IDs shared across modules are planned once, with their sources
combined. The installer builds one package checklist for all selected modules
before installing any packages or applying module configuration. Left/Right
chooses a manager and Space skips a package. After package selection, it
calculates and displays the installation order as numbered module steps, then
asks for final confirmation before making changes. Returning with `b` or
cancelling either prompt does not apply any modules. `--yes` accepts the package
plan and installation order without prompting. The order follows module and
mapping requirements, plus the chosen packages' provider dependencies. Unchosen
sources and skipped packages do not add provider dependencies, and an already
available manager needs no provider-module step. Shared package IDs are installed
only once, before the first module that needs them.

Package installation is skipped when the explicitly declared command is already
in `PATH`. This check happens before package-manager queries and provider
requirements, so tools installed outside a package manager do not require one.
If the command is absent, or `command` is `None`, package-manager records are
still checked to avoid reinstalling an existing package. Source availability and
manager priority remain platform-specific. Skipping a package does not skip the
module's setup, configuration mappings, or applied-state update. Already
available managers are offered regardless of provider module selection. Managers
that will be installed by explicitly selected provider modules are also offered
in the same checklist, without querying them before they exist. Choosing such
a manager orders its provider before dependent modules; installed-package
records are checked once that manager becomes available. A provider cannot use
its own unavailable manager to install its prerequisites. If no declared manager
is available or supplied by a selected module, select a provider module to
install one (for example, `brew` for Homebrew); otherwise installation stops with
the provider IDs to select. `--yes` does not bypass this requirement.

Shell integration requires the corresponding shell module to be selected in
this run or recorded as previously applied: `bash` or `zsh` on Linux, and
`powershell` on Windows. These requirements gate integration files rather than
implicitly selecting or rerunning shell modules. For example, `--modules mise`
copies the Bash activation script when `bash` is recorded, or the Zsh activation
script when `zsh` is recorded, without replacing either profile. If the shell
module is neither selected nor recorded, its integration is skipped. Existing
shell files alone do not satisfy this gate.

Bash, Zsh, and PowerShell profile mappings replace only the section between
their border comments. Bash uses `# >>> dotfiles managed bash section >>>` and
`# <<< dotfiles managed bash section <<<`; Zsh and PowerShell use the same
markers with `zsh` and `powershell` in place of `bash`. If a profile has no
managed section, the installer appends one while preserving the existing
contents; malformed or unmatched borders stop the update without changing the
file. Content-changing replacements are backed up; identical regular files are
left untouched and do not create new backups. Module-local embedded assets are
installed as regular files. Private credential files with matching content have
their permissions checked and repaired in place if needed. See the
[README](../README.md) for package-manager priority and Atuin/OpenCode credential
setup.

## Applied Module State

After each module's setup and file mappings succeed, the installer records its
ID in a local JSON file. Completed modules remain recorded if a later module
fails. The interactive checklist labels these modules as `previously applied`,
but that status does not change default selections or prevent explicitly
selected modules from running again. Discovering or using a manager does not
mark its provider module as previously applied: only successful module setup
and mappings do that. Previously applied modules also satisfy mapping requirements
for selected modules, such as mise's shell activation, without being rerun.

The state file lives beside the backup directory:

- Linux: `$XDG_STATE_HOME/dotfiles/state.json`, falling back to
  `~/.local/state/dotfiles/state.json`.
- Windows: `%LOCALAPPDATA%\dotfiles\state.json`, falling back to
  `~\AppData\Local\dotfiles\state.json`.

```json
{
  "schema_version": 1,
  "modules": ["bash", "git", "mise"]
}
```

`schema_version` describes the file format only. There are no timestamps,
installer versions, or outdated checks. Entries mean the module was previously
applied, not that its packages or files are still present. Modules are recorded
even when package installation is intentionally skipped, as long as setup and
mappings succeed. Existing entries are retained on failed reruns.

State updates use atomic replacement. An OS-managed lock on the adjacent
`state.lock` file prevents concurrent installations from overwriting state and
is released when the installer exits. Invalid or unsupported state causes an
error before installation changes rather than silently discarding records.
State files are local to the user and should not be committed.

## OpenCode Configuration

The OpenCode module writes v2 configuration keys: `providers`, `mcp.servers`, and
`plugins`. It adds `$schema: "https://opencode.ai/config.json"` for editor
validation and migrates legacy MCP-server and plugin entries. When provider
credentials are configured, legacy provider records are converted as well.
Provider credentials remain in protected files and are referenced with
`{file:...}`.

## OpenCode Theme Downloads

OpenCode theme downloads require the installed `curl` executable (`curl.exe`
on Windows). Curl runs directly, without a shell, using its TLS backend and
default certificate and hostname verification. User curlrc files and URL globbing
are disabled, and requests/redirects are restricted to HTTP/HTTPS. Requests have
a 10-second connection timeout, a 60-second total request timeout, and a maximum
of 10 redirects. Theme-tree and individual theme responses are limited
to 1 MiB each, including responses without a declared content length. Downloaded
JSON is validated before managed outputs are considered for replacement. Every
setup rerun still fetches upstream themes, but converged config, credential, and
theme files are not rewritten or backed up.
