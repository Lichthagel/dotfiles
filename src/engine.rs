use crate::cli::{
    confirm_installation_order, select_packages, InstallRequest, PackageSelection, TerminalIo,
};
use crate::error::{InstallError, Result};
use crate::mappings::apply_module_mappings;
use crate::model::Platform;
use crate::packages::{
    build_package_plan_with_providers, install_package, CommandRunner, ManagerAdapter,
    PackageManagerAdapter, PackagePlan,
};
use crate::platform::{FileChange, PlatformOps, TimestampProvider};
use crate::registry::{self, ModuleSpec};
use crate::secrets::{decrypt_embedded, SecretMode, TerminalIdentityPrompt};
use crate::setup::{HttpClient, SetupContext, SetupProbeContext};
use std::collections::{BTreeMap, BTreeSet};

pub struct InstallContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
    pub http: &'a dyn HttpClient,
    pub terminal: &'a mut dyn TerminalIo,
    pub timestamp: &'a dyn TimestampProvider,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallOutcome {
    Completed,
    BackToModuleSelection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Visited,
}

struct Resolver<'a> {
    platform: Platform,
    modules: &'a [ModuleSpec],
    selected: BTreeSet<String>,
    states: BTreeMap<String, VisitState>,
    stack: Vec<String>,
    ordered: Vec<&'a ModuleSpec>,
}

impl<'a> Resolver<'a> {
    fn new(platform: Platform, modules: &'a [ModuleSpec], selected: &[String]) -> Self {
        Self {
            platform,
            modules,
            selected: selected.iter().cloned().collect(),
            states: BTreeMap::new(),
            stack: Vec::new(),
            ordered: Vec::new(),
        }
    }

    fn resolve(&mut self, id: &str) -> Result<()> {
        match self.states.get(id) {
            Some(VisitState::Visited) => return Ok(()),
            Some(VisitState::Visiting) => {
                let start = self.stack.iter().position(|item| item == id).unwrap_or(0);
                let mut cycle = self.stack[start..].to_vec();
                cycle.push(id.to_owned());
                return Err(InstallError::Usage(format!(
                    "module dependency cycle detected: {}",
                    cycle.join(" -> ")
                )));
            }
            None => {}
        }

        let module = self
            .modules
            .iter()
            .find(|module| module.id == id)
            .ok_or_else(|| InstallError::Usage(format!("unknown module dependency: {id}")))?;
        if !module.supports(self.platform) {
            return Err(InstallError::Usage(format!(
                "module is not supported on {:?}: {id}",
                self.platform
            )));
        }

        self.states.insert(id.to_owned(), VisitState::Visiting);
        self.stack.push(id.to_owned());

        for dependency in module.requires.iter().copied() {
            if !self
                .modules
                .iter()
                .any(|candidate| candidate.id == dependency)
            {
                return Err(InstallError::Usage(format!(
                    "unknown module dependency: {dependency}"
                )));
            }
            if !self.selected.iter().any(|selected| selected == dependency) {
                return Err(InstallError::Usage(format!(
                    "module {} requires module {dependency}; explicitly select it in the module checklist or with --modules",
                    module.id
                )));
            }
            self.resolve(dependency)?;
        }

        self.stack.pop();
        self.states.insert(id.to_owned(), VisitState::Visited);
        self.ordered.push(module);
        Ok(())
    }
}

pub fn resolve_modules(
    selected: &[String],
    platform: Platform,
    runner: &dyn CommandRunner,
) -> Result<Vec<&'static ModuleSpec>> {
    resolve_modules_from_registry(selected, platform, runner, registry::modules())
}

pub fn resolve_modules_from_registry<'a>(
    selected: &[String],
    platform: Platform,
    _runner: &dyn CommandRunner,
    modules: &'a [ModuleSpec],
) -> Result<Vec<&'a ModuleSpec>> {
    crate::cli::validate_modules_against(selected, platform, modules)?;
    let mut resolver = Resolver::new(platform, modules, selected);
    for id in selected {
        resolver.resolve(id)?;
    }
    Ok(resolver.ordered)
}

pub fn installation_order<'a>(
    modules: &[&'a ModuleSpec],
    package_plan: &PackagePlan,
    platform: Platform,
    runner: &dyn CommandRunner,
) -> Result<Vec<&'a ModuleSpec>> {
    let by_id = modules
        .iter()
        .map(|module| (module.id, *module))
        .collect::<BTreeMap<_, _>>();
    let mut emitted = BTreeSet::new();
    let mut order = Vec::with_capacity(modules.len());

    while emitted.len() < modules.len() {
        let mut progress = false;
        for module in modules {
            if emitted.contains(module.id) {
                continue;
            }
            let mut dependencies = module.requires.to_vec();
            dependencies.extend(
                module
                    .mappings
                    .iter()
                    .filter(|mapping| mapping.platform == platform)
                    .flat_map(|mapping| mapping.requires.iter().copied()),
            );
            let mut provider_dependencies = package_plan
                .actions()
                .iter()
                .filter(|action| {
                    action.selected
                        && module
                            .packages
                            .iter()
                            .any(|package| package.id == action.package_id)
                        && !PackageManagerAdapter::new(platform, action.manager).available(runner)
                })
                .filter_map(|action| {
                    by_id.values().find(|candidate| {
                        candidate.provides == Some(action.manager) && candidate.id != module.id
                    })
                });
            let dependencies_ready = dependencies
                .iter()
                .all(|dependency| !by_id.contains_key(dependency) || emitted.contains(*dependency));
            let providers_ready =
                provider_dependencies.all(|provider| emitted.contains(provider.id));
            if dependencies_ready && providers_ready {
                emitted.insert(module.id);
                order.push(*module);
                progress = true;
            }
        }
        if !progress {
            let remaining = modules
                .iter()
                .filter(|module| !emitted.contains(module.id))
                .map(|module| module.id)
                .collect::<Vec<_>>();
            return Err(InstallError::Usage(format!(
                "module dependency cycle detected while ordering: {}",
                remaining.join(", ")
            )));
        }
    }
    Ok(order)
}

pub fn install(request: InstallRequest, context: InstallContext<'_>) -> Result<InstallOutcome> {
    install_with_registry(request, context, registry::modules())
}

pub fn install_with_registry(
    request: InstallRequest,
    context: InstallContext<'_>,
    modules: &[ModuleSpec],
) -> Result<InstallOutcome> {
    if request.modules.is_empty() {
        return Ok(InstallOutcome::Completed);
    }
    if request.platform != context.platform.platform() {
        return Err(InstallError::Usage(format!(
            "request targets {:?}, but the active platform is {:?}",
            request.platform,
            context.platform.platform()
        )));
    }

    let resolved =
        resolve_modules_from_registry(&request.modules, request.platform, context.runner, modules)?;
    let mut state = crate::state::StateStore::open(context.platform)?;
    let mut package_plan = build_package_plan_with_providers(
        context.runner,
        request.platform,
        &resolved,
        modules,
        request.yes,
    )?;
    if !request.yes {
        match select_packages(&mut package_plan, context.terminal)? {
            PackageSelection::Continue => {}
            PackageSelection::Back if request.allow_module_back => {
                return Ok(InstallOutcome::BackToModuleSelection);
            }
            PackageSelection::Back => {
                return Err(InstallError::Usage(
                    "module selection cannot return with explicit --modules".into(),
                ));
            }
            PackageSelection::Cancel => return Err(InstallError::Cancelled),
        }
    }
    let order = installation_order(&resolved, &package_plan, request.platform, context.runner)?;
    if !request.yes {
        confirm_installation_order(&order, context.terminal)?;
    }
    // Applied modules satisfy mapping gates without joining the installation plan.
    let mapping_modules = modules
        .iter()
        .filter(|module| {
            module.supports(request.platform)
                && (state.contains(module.id)
                    || resolved.iter().any(|selected| selected.id == module.id))
        })
        .map(|module| module.id.to_owned())
        .collect::<Vec<_>>();
    let deferred_packages = package_plan
        .actions()
        .iter()
        .filter(|action| {
            !PackageManagerAdapter::new(request.platform, action.manager).available(context.runner)
        })
        .map(|action| action.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut handled_packages = BTreeSet::new();
    for module in order {
        for action in package_plan.actions().iter().filter(|action| {
            module
                .packages
                .iter()
                .any(|package| package.id == action.package_id)
        }) {
            if !handled_packages.insert(&action.package_id) {
                continue;
            }
            if action.selected {
                let command_available =
                    resolved
                        .iter()
                        .flat_map(|module| module.packages)
                        .any(|package| {
                            package.id == action.package_id
                                && package
                                    .command
                                    .is_some_and(|command| context.runner.available(command))
                        });
                if command_available {
                    continue;
                }
                let adapter = PackageManagerAdapter::new(request.platform, action.manager);
                if !adapter.available(context.runner) {
                    return Err(InstallError::Package(format!(
                        "{} package {}: selected manager {} is still unavailable after its provider module runs",
                        action.module, action.package_id, action.manager.label()
                    )));
                }
                // Managers installed by an earlier module could not be queried during planning.
                if deferred_packages.contains(action.package_id.as_str())
                    && adapter.installed(context.runner, &action.package_name)?
                {
                    continue;
                }
                install_package(context.runner, request.platform, action).map_err(|error| {
                    InstallError::Package(format!(
                        "{} package {}: {error}",
                        action.module, action.package_id
                    ))
                })?;
                context.runner.refresh().map_err(|error| {
                    InstallError::Package(format!(
                        "{} package {}: command lookup refresh failed: {error}",
                        action.module, action.package_id
                    ))
                })?;
            } else {
                context.terminal.write_line(&format!(
                    "Skipped package {} for module {}; dotfiles were still installed.",
                    action.package_id, action.module
                ))?;
            }
        }

        let probe_context = SetupProbeContext {
            platform: context.platform,
            runner: context.runner,
        };
        let requested_secret_mode = module
            .secret_request
            .map(|request| request(&probe_context))
            .transpose()?
            .flatten();
        let secret_scope = match (module.secret_bundle, requested_secret_mode) {
            (Some(bundle), Some(mode)) => {
                let mut prompt = (mode == SecretMode::Optional
                    || context.terminal.is_interactive())
                .then(|| TerminalIdentityPrompt::new(context.terminal));
                decrypt_embedded(
                    bundle,
                    mode,
                    prompt
                        .as_mut()
                        .map(|prompt| prompt as &mut dyn crate::secrets::IdentityPrompt),
                )?
            }
            _ => None,
        };
        let setup_context = SetupContext {
            platform: context.platform,
            runner: context.runner,
            http: context.http,
            secrets: secret_scope.as_ref().map(|scope| scope.values()),
        };
        if let Some(setup) = module.setup {
            setup(&setup_context)
                .map_err(|error| InstallError::Setup(format!("{}: {error}", module.id)))?;
        }
        let backups = apply_module_mappings(
            context.platform,
            module,
            &mapping_modules,
            context.timestamp,
        )
        .map_err(|error| InstallError::Mapping(format!("{}: {error}", module.id)))?;
        state.record(module.id).map_err(|error| {
            InstallError::State(format!(
                "could not record completed module {}: {error}",
                module.id
            ))
        })?;
        for applied in backups {
            match applied.change {
                FileChange::Content { backup } => {
                    if let Some(path) = backup {
                        context.terminal.write_line(&format!(
                            "backed up: {} -> {}",
                            applied.target.display(),
                            path.display()
                        ))?;
                    }
                    context
                        .terminal
                        .write_line(&format!("installed: {}", applied.target.display()))?;
                }
                FileChange::Unchanged => context
                    .terminal
                    .write_line(&format!("unchanged: {}", applied.target.display()))?,
                FileChange::Permissions => context
                    .terminal
                    .write_line(&format!("secured: {}", applied.target.display()))?,
            }
        }
        context
            .terminal
            .write_line(&format!("completed module: {}", module.id))?;
    }
    Ok(InstallOutcome::Completed)
}
