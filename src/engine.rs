use crate::cli::{select_packages, InstallRequest, PackageSelection, TerminalIo};
use crate::error::{InstallError, Result};
use crate::mappings::apply_module_mappings;
use crate::model::Platform;
use crate::packages::{
    build_package_plan, install_package, CommandRunner, ManagerAdapter, PackageManagerAdapter,
};
use crate::platform::{PlatformOps, TimestampProvider};
use crate::registry::{self, ModuleSpec};
use crate::secrets::{decrypt_embedded, TerminalIdentityPrompt};
use crate::setup::{run_setup, secret_mode, HttpClient, SetupContext};
use std::collections::{BTreeMap, BTreeSet};

pub struct InstallContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
    pub http: &'a dyn HttpClient,
    pub terminal: &'a mut dyn TerminalIo,
    pub timestamp: &'a dyn TimestampProvider,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Visited,
}

struct Resolver<'a, 'r> {
    platform: Platform,
    runner: &'r dyn CommandRunner,
    modules: &'a [ModuleSpec],
    states: BTreeMap<String, VisitState>,
    stack: Vec<String>,
    ordered: Vec<&'a ModuleSpec>,
}

impl<'a, 'r> Resolver<'a, 'r> {
    fn new(platform: Platform, runner: &'r dyn CommandRunner, modules: &'a [ModuleSpec]) -> Self {
        Self {
            platform,
            runner,
            modules,
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

        let mut dependencies = module.requires.to_vec();
        dependencies.extend(
            module
                .mappings
                .iter()
                .filter(|mapping| mapping.platform == self.platform)
                .flat_map(|mapping| mapping.requires.iter().copied()),
        );
        for dependency in dependencies {
            self.resolve(dependency)?;
        }

        let logical_names = module
            .packages
            .iter()
            .map(|package| package.logical)
            .collect::<BTreeSet<_>>();
        for logical in logical_names {
            let candidates = module.packages.iter().filter(|package| {
                package.logical == logical && package.manager.is_supported_on(self.platform)
            });
            if candidates.clone().any(|package| {
                PackageManagerAdapter::new(self.platform, package.manager).available(self.runner)
            }) {
                continue;
            }
            for package in candidates {
                let provider = self.modules.iter().find(|candidate| {
                    candidate.provides == Some(package.manager) && candidate.supports(self.platform)
                });
                if let Some(provider) = provider {
                    self.resolve(provider.id)?;
                    break;
                }
            }
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
    runner: &dyn CommandRunner,
    modules: &'a [ModuleSpec],
) -> Result<Vec<&'a ModuleSpec>> {
    crate::cli::validate_modules_against(selected, platform, modules)?;
    let mut resolver = Resolver::new(platform, runner, modules);
    for id in selected {
        resolver.resolve(id)?;
    }
    Ok(resolver.ordered)
}

pub fn ordered_phases<'a>(modules: &[&'a ModuleSpec]) -> Result<Vec<Vec<&'a ModuleSpec>>> {
    let by_id = modules
        .iter()
        .map(|module| (module.id, *module))
        .collect::<BTreeMap<_, _>>();
    let mut emitted = BTreeSet::new();
    let mut phases = Vec::with_capacity(modules.len());

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
                    .flat_map(|mapping| mapping.requires.iter().copied()),
            );
            let mut provider_dependencies = module.packages.iter().filter_map(|package| {
                by_id.values().find(|candidate| {
                    candidate.provides == Some(package.manager) && candidate.id != module.id
                })
            });
            let dependencies_ready = dependencies
                .iter()
                .all(|dependency| !by_id.contains_key(dependency) || emitted.contains(*dependency));
            let providers_ready =
                provider_dependencies.all(|provider| emitted.contains(provider.id));
            if dependencies_ready && providers_ready {
                emitted.insert(module.id);
                phases.push(vec![*module]);
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
    Ok(phases)
}

pub fn install(request: InstallRequest, context: InstallContext<'_>) -> Result<()> {
    install_with_registry(request, context, registry::modules())
}

pub fn install_with_registry(
    request: InstallRequest,
    context: InstallContext<'_>,
    modules: &[ModuleSpec],
) -> Result<()> {
    if request.modules.is_empty() {
        return Ok(());
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
    let resolved_ids = resolved
        .iter()
        .map(|module| module.id.to_owned())
        .collect::<Vec<_>>();

    for phase in ordered_phases(&resolved)? {
        let mut package_plan =
            build_package_plan(context.runner, request.platform, &phase, request.yes)?;
        if !request.yes {
            match select_packages(&mut package_plan, context.terminal)? {
                PackageSelection::Continue => {}
                PackageSelection::Back => {
                    return Err(InstallError::Usage(
                        "module selection is fixed by --modules".into(),
                    ))
                }
                PackageSelection::Cancel => return Err(InstallError::Cancelled),
            }
        }
        for action in package_plan.actions() {
            if action.selected {
                install_package(context.runner, request.platform, action).map_err(|error| {
                    InstallError::Package(format!(
                        "{} package {}: {error}",
                        action.module, action.logical
                    ))
                })?;
                context.runner.refresh().map_err(|error| {
                    InstallError::Package(format!(
                        "{} package {}: command lookup refresh failed: {error}",
                        action.module, action.logical
                    ))
                })?;
            } else {
                context.terminal.write_line(&format!(
                    "Skipped package {} for module {}; dotfiles were still installed.",
                    action.logical, action.module
                ))?;
            }
        }

        let module = phase[0];
        let secret_scope =
            if let (Some(bundle), Some(mode)) = (module.secret_bundle, secret_mode(module.setup)) {
                let mut prompt = context
                    .terminal
                    .is_interactive()
                    .then(|| TerminalIdentityPrompt::new(context.terminal));
                decrypt_embedded(
                    bundle,
                    mode,
                    prompt
                        .as_mut()
                        .map(|prompt| prompt as &mut dyn crate::secrets::IdentityPrompt),
                )?
            } else {
                None
            };
        let setup_context = SetupContext {
            platform: context.platform,
            runner: context.runner,
            http: context.http,
            secrets: secret_scope.as_ref().map(|scope| scope.values()),
        };
        run_setup(module.setup, &setup_context)
            .map_err(|error| InstallError::Setup(format!("{}: {error}", module.id)))?;
        apply_module_mappings(context.platform, module, &resolved_ids, context.timestamp)
            .map_err(|error| InstallError::Mapping(format!("{}: {error}", module.id)))?;
    }
    Ok(())
}
