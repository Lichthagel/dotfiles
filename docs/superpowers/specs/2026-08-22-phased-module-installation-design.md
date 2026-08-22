# Phased Module Installation

## Goal

Allow a selected module to provide a package manager that is needed by other selected modules. The provider module is a soft dependency: it is added to the installation plan when needed, runs first, and is not required in the user's explicit module list. The first provider is `mise`; the mechanism must support additional provider modules later.

## Module Metadata

Modules may declare `provides=<manager>` to identify a package manager they install and configure. The `mise` module will declare `provides=mise`.

Package declarations already identify their manager (`package=<logical>|<manager>:<name>`). A selected package that resolves to a manager with a matching provider implicitly requires that provider module. Consumer modules must not add `requires=mise` solely for package installation.

Explicit module dependencies use `requires=<module[,module...]>` at module scope and are hard ordering requirements for future modules. They are distinct from a provider dependency: provider dependencies are inferred from package declarations and are soft additions to the selected set.

## Resolution And Phases

1. Parse the manifest and module metadata, including `provides`.
2. Start with the modules explicitly selected by the user.
3. Inspect package declarations for the current module set before selecting managers. If a package has a declaration for an unavailable manager with a matching provider module, add that provider to the module set if it is not already present. This inference happens before reporting that the package has no available manager.
4. Resolve provider modules before their consumers using a dependency graph. A provider's own package must not be assigned to the manager it provides; its declarations must include a bootstrap-capable manager such as `apt`, `dnf`, `pacman`, `brew`, `winget`, or `scoop`.
5. After each provider phase makes a manager available, rebuild candidates for remaining modules and add any newly discovered providers.
6. For each phase in dependency order:
   - Build the package plan for modules in that phase.
   - Apply the existing interactive package selection and confirmation behavior.
   - Install selected packages.
   - Run setup scripts for the phase's modules.
   - Install mappings for the phase's modules.
7. Recompute package-manager availability after each phase. Packages that become available through a newly installed provider are handled in a later phase.

The provider module is considered processed after its package, setup, and mappings succeed. If a provider cannot be installed, dependent phases are not attempted and no dependent module mappings are installed.

## Selection Semantics

Explicit `--apps`/`-Apps` remains the source of user-selected modules. Soft provider modules are internal additions and do not change explicit-module validation or interactive module selection semantics. The provider may still appear in package and setup output so users can understand what is being installed.

Default interactive selections are resolved after the initial checklist and before package planning. A default provider is not installed merely because it is default; it is installed when selected explicitly or inferred as a provider for a selected package.

## Compatibility And Errors

- Existing module configuration keys and platform-qualified declarations remain valid.
- A missing provider, duplicate provider, unsupported provider manager, or provider cycle is a manifest error with a clear module/provider message.
- A package with no available manager remains an error after provider resolution.
- Existing `--yes`/`-Yes` behavior applies independently to each phase. Noninteractive package planning still requires the confirmation bypass.
- Existing backup and mapping behavior is retained, but mappings are applied only after the owning phase's packages and setup succeed.

## Testing

Add coverage for both installers that verifies:

- A selected module using `mise` implicitly adds the `mise` provider.
- The provider is installed through a system manager before dependent packages.
- A later phase can use the newly installed provider.
- Provider setup and mappings run before dependent module setup and mappings.
- Provider self-install through its own manager is rejected or avoided.
- Provider installation failure prevents dependent work.
- Multiple providers/dependency ordering is deterministic and cycle errors are reported.
- Existing explicit selection, package confirmation, mapping backup, and platform parser behavior remain unchanged.
