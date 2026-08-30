use crate::cli::TerminalIo;
use crate::cli::{select_packages, InstallRequest, PackageSelection};
use crate::error::{InstallError, Result};
use crate::mappings::apply_module_mappings;
use crate::model::Platform;
use crate::packages::{build_package_plan, install_package, CommandRunner, ManagerAdapter};
use crate::platform::PlatformOps;
use crate::registry::{self, ModuleSpec};
use crate::secrets::{decrypt_embedded, TerminalIdentityPrompt};
use crate::setup::{run_setup, secret_mode, HttpClient, SetupContext};
use std::collections::BTreeSet;

pub struct InstallContext<'a> {
    pub platform: &'a dyn PlatformOps,
    pub runner: &'a dyn CommandRunner,
    pub http: &'a dyn HttpClient,
    pub terminal: &'a mut dyn TerminalIo,
}

pub fn resolve_modules(
    selected: &[String],
    platform: Platform,
    runner: &dyn CommandRunner,
) -> Result<Vec<&'static ModuleSpec>> {
    crate::cli::validate_modules(selected, platform)?;
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    for id in selected {
        add_module(id, platform, &mut resolved, &mut seen)?;
    }

    let mut index = 0;
    while index < resolved.len() {
        let module = resolved[index];
        for requirement in module.requires.iter().chain(
            module
                .mappings
                .iter()
                .filter(|mapping| mapping.platform == platform)
                .flat_map(|mapping| mapping.requires),
        ) {
            add_module(requirement, platform, &mut resolved, &mut seen)?;
        }
        let logical_names = module
            .packages
            .iter()
            .map(|package| package.logical)
            .collect::<BTreeSet<_>>();
        for logical in logical_names {
            let candidates = module.packages.iter().filter(|package| {
                package.logical == logical && package.manager.is_supported_on(platform)
            });
            if candidates.clone().any(|package| {
                crate::packages::PackageManagerAdapter::new(platform, package.manager)
                    .available(runner)
            }) {
                continue;
            }
            for package in candidates {
                let Some(provider) = registry::modules().iter().find(|candidate| {
                    candidate.provides == Some(package.manager) && candidate.supports(platform)
                }) else {
                    continue;
                };
                add_module(provider.id, platform, &mut resolved, &mut seen)?;
                break;
            }
        }
        index += 1;
    }
    Ok(resolved)
}

fn add_module(
    id: &str,
    platform: Platform,
    resolved: &mut Vec<&'static ModuleSpec>,
    seen: &mut BTreeSet<String>,
) -> Result<()> {
    if seen.contains(id) {
        return Ok(());
    }
    let module = registry::find_module(id)
        .ok_or_else(|| InstallError::Usage(format!("unknown module dependency: {id}")))?;
    if !module.supports(platform) {
        return Err(InstallError::Usage(format!(
            "module is not supported on {platform:?}: {id}"
        )));
    }
    seen.insert(id.to_owned());
    resolved.push(module);
    Ok(())
}

pub fn ordered_phases(modules: &[&'static ModuleSpec]) -> Vec<Vec<&'static ModuleSpec>> {
    let mut ordered = Vec::with_capacity(modules.len());
    let providers = modules
        .iter()
        .copied()
        .filter(|module| module.provides.is_some());
    let nonproviders = modules
        .iter()
        .copied()
        .filter(|module| module.provides.is_none());
    for module in providers.chain(nonproviders) {
        ordered.push(vec![module]);
    }
    ordered
}

pub fn install(request: InstallRequest, context: InstallContext<'_>) -> Result<()> {
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

    let resolved = resolve_modules(&request.modules, request.platform, context.runner)?;
    let resolved_ids = resolved
        .iter()
        .map(|module| module.id.to_owned())
        .collect::<Vec<_>>();

    for phase in ordered_phases(&resolved) {
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
        apply_module_mappings(context.platform, module, &resolved_ids)
            .map_err(|error| InstallError::Mapping(format!("{}: {error}", module.id)))?;
    }
    Ok(())
}
