# Installation

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
selection and diagnostics; it does not determine the executable name. Package
IDs shared across modules are planned once, with their sources combined. The
installer builds one package checklist for all selected modules before installing
any packages or applying module configuration. Left/Right chooses a manager and
Space skips a package. After package selection, it calculates and displays the
installation order as numbered module steps, then asks for final confirmation
before making changes. Returning with `b` or cancelling either prompt does not
apply any modules. `--yes` accepts the package plan and installation order
without prompting. The order follows module and mapping requirements, plus the
chosen packages' provider dependencies. Unchosen sources and skipped packages
do not add provider dependencies, and an already available manager needs no
provider-module step. Shared package IDs are installed only once, before the
first module that needs them.

Package installation is skipped when the explicitly declared command is already
in `PATH`. This check happens before package-manager queries and provider
requirements, so tools installed outside a package manager do not require one.
If the command is absent, or `command` is `None`, package-manager records are
still checked to avoid reinstalling an existing package. Source availability and
manager priority remain platform-specific. Skipping a package does not skip the
module's setup, configuration mappings, or applied-state update. Already
available managers are offered regardless of provider module selection. Managers
that will be installed by explicitly selected provider modules are also offered
in the same checklist, without querying them before they exist. Choosing such a
manager orders its provider before dependent modules; installed-package records
are checked once that manager becomes available. A provider cannot use its own
unavailable manager to install its prerequisites. If no declared manager is
available or supplied by a selected module, select a provider module to install
one (for example, `brew` for Homebrew); otherwise installation stops with the
provider IDs to select. `--yes` does not bypass this requirement.

Shell integration requires the corresponding shell module to be selected in
this run or recorded as previously applied: `bash` or `zsh` on Linux, and
`powershell` on Windows. These requirements gate integration files rather than
implicitly selecting or rerunning shell modules. If the shell module is neither
selected nor recorded, its integration is skipped. Existing shell files alone do
not satisfy this gate.

Bash, Zsh, and PowerShell profile mappings replace only the section between
their border comments. If a profile has no managed section, the installer
appends one while preserving existing contents; malformed or unmatched borders
stop the update without changing the file. Content-changing replacements are
backed up; identical regular files are left untouched and do not create new
backups. Module-local embedded assets are installed as regular files. Private
credential files with matching content have their permissions checked and
repaired in place if needed. See the [README](../README.md) for package-manager
priority and credential setup.

## Applied Module State

After each module's setup and file mappings succeed, the installer records its
ID in a local JSON file. Completed modules remain recorded if a later module
fails. The interactive checklist labels these modules as `previously applied`,
but that status does not change default selections or prevent explicitly
selected modules from running again. Discovering or using a manager does not
mark its provider module as previously applied: only successful module setup
and mappings do that. Previously applied modules also satisfy mapping
requirements for selected modules, such as mise's shell activation, without
being rerun.

The state file lives beside the backup directory:

- Linux: `$XDG_STATE_HOME/dotfiles/state.json`, falling back to
  `~/.local/state/dotfiles/state.json`.
- Windows: `%LOCALAPPDATA%\\dotfiles\\state.json`, falling back to
  `~\\AppData\\Local\\dotfiles\\state.json`.

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
