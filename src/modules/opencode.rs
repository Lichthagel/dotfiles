use super::{CROSS_PLATFORM, NO_REQUIREMENTS};
use crate::error::{InstallError, Result};
use crate::model::{EmbeddedSecretBundle, PackageManager, Platform};
use crate::platform::PlatformOps;
use crate::registry::{ModuleSpec, Package, PackageSource};
use crate::secrets::{SecretMode, SecretValues};
use crate::setup::http::MAX_THEME_FILE_BYTES;
use crate::setup::jsonc::JsoncDocument;
use crate::setup::{SetupContext, SetupProbeContext};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const OPENCODE_SECRET_BUNDLE: EmbeddedSecretBundle =
    EmbeddedSecretBundle::new("OpenCode", include_bytes!("../../secrets/opencode.env.age"));

const OPENCODE_PACKAGES: &[Package] = &[Package {
    id: "opencode",
    command: Some("opencode"),
    sources: &[
        PackageSource {
            manager: PackageManager::Brew,
            name: "anomalyco/tap/opencode",
        },
        PackageSource {
            manager: PackageManager::Mise,
            name: "github:anomalyco/opencode",
        },
        PackageSource {
            manager: PackageManager::Scoop,
            name: "opencode",
        },
    ],
}];

pub const SPEC: ModuleSpec = ModuleSpec {
    id: "opencode",
    description: "OpenCode AI coding agent",
    platforms: CROSS_PLATFORM,
    default_selected: false,
    provides: None,
    requires: NO_REQUIREMENTS,
    packages: OPENCODE_PACKAGES,
    mappings: &[],
    setup: Some(configure_opencode),
    secret_request: Some(secret_request),
    secret_bundle: Some(OPENCODE_SECRET_BUNDLE),
};

pub const SUPERPOWERS_PLUGIN: &str = "superpowers@git+https://github.com/obra/superpowers.git";
const OPENCODE_CONFIG_SCHEMA: &str = "https://opencode.ai/config.json";
const REQUIRED_PROVIDER_SECRETS: [&str; 4] = [
    "OPENROUTER_API_KEY",
    "AZURE_API_KEY",
    "AZURE_RESOURCE_NAME",
    "DIGITALOCEAN_ACCESS_TOKEN",
];
const THEME_API_URL: &str =
    "https://api.github.com/repos/catppuccin/opencode/git/trees/main?recursive=1";
const THEME_RAW_URL: &str = "https://raw.githubusercontent.com/catppuccin/opencode/main";
const MAX_THEME_FILES: usize = 256;
const MAX_THEME_TOTAL_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeFile {
    pub filename: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ThemeTree {
    pub files: Vec<ThemeFile>,
}

pub fn secret_request(_probe: &SetupProbeContext<'_>) -> Result<Option<SecretMode>> {
    Ok(Some(SecretMode::Optional))
}

pub fn configure_opencode(context: &SetupContext<'_>) -> Result<()> {
    let configure_providers = provider_values(context.secrets)?;
    let config_dir = opencode_config_dir(context.platform)?;
    let config_file = config_path(context.platform, &config_dir);
    let mut document = if config_file.exists() {
        let source = fs::read_to_string(&config_file)?;
        JsoncDocument::parse(&source)?
    } else {
        JsoncDocument::empty()
    };

    update_configuration(document.value_mut(), &config_dir, configure_providers)?;
    let config_bytes = document.to_bytes()?;
    JsoncDocument::parse(std::str::from_utf8(&config_bytes).map_err(|error| {
        InstallError::Setup(format!("generated OpenCode config is not UTF-8: {error}"))
    })?)?;

    let api_url =
        std::env::var("OPENCODE_THEME_API_URL").unwrap_or_else(|_| THEME_API_URL.to_owned());
    let raw_url =
        std::env::var("OPENCODE_THEME_RAW_URL").unwrap_or_else(|_| THEME_RAW_URL.to_owned());
    let themes = fetch_themes(context, &api_url, &raw_url)?;
    fs::create_dir_all(&config_dir)?;

    let stage_dir = tempfile::tempdir_in(&config_dir).map_err(|error| {
        InstallError::Setup(format!("create OpenCode staging directory: {error}"))
    })?;
    let staged_config = stage_dir.path().join(config_filename(context.platform));
    fs::write(&staged_config, &config_bytes)?;
    JsoncDocument::parse(
        std::str::from_utf8(&fs::read(&staged_config)?).map_err(|error| {
            InstallError::Setup(format!("staged OpenCode config is not UTF-8: {error}"))
        })?,
    )?;

    let mut staged_credentials = Vec::new();
    if configure_providers {
        let values = context
            .secrets
            .expect("provider validation requires secret values");
        let credentials = [
            ("openrouter-api-key", "OPENROUTER_API_KEY"),
            ("azure-api-key", "AZURE_API_KEY"),
            ("azure-resource-name", "AZURE_RESOURCE_NAME"),
            ("digitalocean-access-token", "DIGITALOCEAN_ACCESS_TOKEN"),
        ];
        for (filename, secret_name) in credentials {
            let value = values.get(secret_name).expect("validated provider secret");
            let mut bytes = Zeroizing::new(Vec::with_capacity(value.len()));
            bytes.extend_from_slice(value.as_bytes());
            staged_credentials.push((filename, bytes));
        }
    }

    let staged_themes = stage_themes(stage_dir.path(), &themes)?;

    context
        .platform
        .ensure_file(&config_file, &fs::read(&staged_config)?)?;
    for (filename, bytes) in staged_credentials {
        context
            .platform
            .ensure_private_file(&config_dir.join(filename), &bytes)?;
    }
    let themes_dir = config_dir.join("themes");
    fs::create_dir_all(&themes_dir)?;
    for (filename, bytes) in staged_themes {
        context
            .platform
            .ensure_file(&themes_dir.join(filename), &bytes)?;
    }
    Ok(())
}

fn provider_values(secrets: Option<&SecretValues>) -> Result<bool> {
    let Some(secrets) = secrets else {
        return Ok(false);
    };
    let configured = REQUIRED_PROVIDER_SECRETS
        .iter()
        .filter(|name| secrets.get(name).is_some_and(|value| !value.is_empty()))
        .count();
    if configured == 0 {
        return Ok(false);
    }
    if configured != REQUIRED_PROVIDER_SECRETS.len() {
        let missing: Vec<_> = REQUIRED_PROVIDER_SECRETS
            .iter()
            .copied()
            .filter(|name| secrets.get(name).is_none_or(str::is_empty))
            .collect();
        return Err(InstallError::Secret(format!(
            "required OpenCode provider secrets are missing or empty: {}",
            missing.join(", ")
        )));
    }
    Ok(true)
}

fn update_configuration(
    value: &mut Value,
    config_dir: &Path,
    configure_providers: bool,
) -> Result<()> {
    let object = value.as_object_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode configuration root must be a JSON object".into())
    })?;

    object.insert(
        "$schema".into(),
        Value::String(OPENCODE_CONFIG_SCHEMA.to_owned()),
    );

    if configure_providers {
        migrate_legacy_providers(object)?;
        set_provider_setting(
            object,
            "openrouter",
            "apiKey",
            format!(
                "{{file:{}}}",
                config_dir.join("openrouter-api-key").display()
            ),
        )?;
        set_provider_setting(
            object,
            "azure",
            "apiKey",
            format!("{{file:{}}}", config_dir.join("azure-api-key").display()),
        )?;
        set_provider_setting(
            object,
            "azure",
            "resourceName",
            format!(
                "{{file:{}}}",
                config_dir.join("azure-resource-name").display()
            ),
        )?;
        set_provider_setting(
            object,
            "digitalocean",
            "apiKey",
            format!(
                "{{file:{}}}",
                config_dir.join("digitalocean-access-token").display()
            ),
        )?;
    }

    let mcp = object
        .entry("mcp".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp = mcp.as_object_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode mcp configuration must be an object".into())
    })?;
    let mut legacy_servers = Map::new();
    let legacy_keys: Vec<_> = mcp
        .keys()
        .filter(|key| !matches!(key.as_str(), "timeout" | "servers"))
        .cloned()
        .collect();
    for key in legacy_keys {
        if let Some(server) = mcp.remove(&key) {
            legacy_servers.insert(key, server);
        }
    }
    let servers = mcp
        .entry("servers".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let servers = servers.as_object_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode MCP servers configuration must be an object".into())
    })?;
    for (name, server) in legacy_servers {
        servers.entry(name).or_insert(server);
    }
    servers.insert(
        "ddg-search".into(),
        serde_json::json!({
            "type": "local",
            "command": ["uvx", "duckduckgo-mcp-server"],
            "environment": {"DDG_SAFE_SEARCH": "OFF"}
        }),
    );

    migrate_legacy_plugins(object)?;
    let plugins = object
        .entry("plugins".to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    let plugins = plugins.as_array_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode plugins configuration must be an array".into())
    })?;
    plugins.retain(|plugin| plugin.as_str() != Some(SUPERPOWERS_PLUGIN));
    plugins.push(Value::String(SUPERPOWERS_PLUGIN.to_owned()));
    Ok(())
}

fn migrate_legacy_plugins(object: &mut Map<String, Value>) -> Result<()> {
    let Some(legacy) = object.remove("plugin") else {
        return Ok(());
    };
    let legacy = legacy.as_array().ok_or_else(|| {
        InstallError::Setup("OpenCode legacy plugin configuration must be an array".into())
    })?;
    let plugins = object
        .entry("plugins".to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    let plugins = plugins.as_array_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode plugins configuration must be an array".into())
    })?;
    let mut migrated = legacy.iter().cloned().map(|plugin| match plugin {
        Value::Array(mut parts) if parts.len() == 2 && parts[0].is_string() => {
            let options = parts.pop().expect("two plugin tuple elements");
            let package = parts.pop().expect("two plugin tuple elements");
            serde_json::json!({"package": package, "options": options})
        }
        plugin => plugin,
    });
    let mut combined: Vec<_> = migrated.by_ref().collect();
    combined.append(plugins);
    *plugins = combined;
    Ok(())
}

fn migrate_legacy_providers(object: &mut Map<String, Value>) -> Result<()> {
    let Some(legacy) = object.remove("provider") else {
        return Ok(());
    };
    let legacy = legacy.as_object().ok_or_else(|| {
        InstallError::Setup("OpenCode legacy provider configuration must be an object".into())
    })?;
    let providers = object
        .entry("providers".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let providers = providers.as_object_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode providers configuration must be an object".into())
    })?;
    for (id, legacy_config) in legacy {
        let mut migrated = legacy_config.clone();
        let config = migrated.as_object_mut().ok_or_else(|| {
            InstallError::Setup(format!(
                "OpenCode provider {id} configuration must be an object"
            ))
        })?;
        if let Some(package) = config.remove("npm") {
            config.entry("package").or_insert(package);
        }
        if let Some(options) = config.remove("options") {
            let options = options.as_object().ok_or_else(|| {
                InstallError::Setup(format!("OpenCode provider {id} options must be an object"))
            })?;
            let settings = config
                .entry("settings".to_owned())
                .or_insert_with(|| Value::Object(Map::new()));
            let settings = settings.as_object_mut().ok_or_else(|| {
                InstallError::Setup(format!("OpenCode provider {id} settings must be an object"))
            })?;
            for (key, value) in options {
                settings.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
        let migrated_config = config.clone();
        let current = providers
            .entry(id.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        let current = current.as_object_mut().ok_or_else(|| {
            InstallError::Setup(format!(
                "OpenCode provider {id} configuration must be an object"
            ))
        })?;
        merge_provider_defaults(current, &migrated_config);
    }
    Ok(())
}

fn merge_provider_defaults(target: &mut Map<String, Value>, defaults: &Map<String, Value>) {
    for (key, value) in defaults {
        if key == "settings" {
            if let (Some(target_settings), Some(default_settings)) = (
                target.get_mut(key).and_then(Value::as_object_mut),
                value.as_object(),
            ) {
                for (setting, default) in default_settings {
                    target_settings
                        .entry(setting.clone())
                        .or_insert_with(|| default.clone());
                }
                continue;
            }
        }
        target.entry(key.clone()).or_insert_with(|| value.clone());
    }
}

fn set_provider_setting(
    object: &mut Map<String, Value>,
    provider_id: &str,
    setting_name: &str,
    setting_value: String,
) -> Result<()> {
    let providers = object
        .entry("providers".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let providers = providers.as_object_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode providers configuration must be an object".into())
    })?;
    let provider = providers
        .entry(provider_id.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let provider = provider.as_object_mut().ok_or_else(|| {
        InstallError::Setup(format!(
            "OpenCode provider {provider_id} configuration must be an object"
        ))
    })?;
    let settings = provider
        .entry("settings".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let settings = settings.as_object_mut().ok_or_else(|| {
        InstallError::Setup(format!(
            "OpenCode provider {provider_id} settings must be an object"
        ))
    })?;
    settings.insert(setting_name.to_owned(), Value::String(setting_value));
    Ok(())
}

pub fn fetch_themes(context: &SetupContext<'_>, api_url: &str, raw_url: &str) -> Result<ThemeTree> {
    let tree = context.http.get_json(api_url)?;
    let entries = tree
        .get("tree")
        .and_then(Value::as_array)
        .ok_or_else(|| InstallError::Setup("Catppuccin theme response has no tree".into()))?;
    if entries.len() > MAX_THEME_FILES * 4 {
        return Err(InstallError::Setup(format!(
            "Catppuccin theme tree contains too many entries: {}",
            entries.len()
        )));
    }
    let mut filenames = BTreeSet::new();
    let mut files = Vec::new();
    let mut total_bytes = 0usize;
    for entry in entries {
        let Some(path) = entry.get("path").and_then(Value::as_str) else {
            continue;
        };
        let is_theme = entry.get("type").and_then(Value::as_str) == Some("blob")
            && path.starts_with("themes/")
            && path.ends_with(".json");
        if !is_theme {
            continue;
        }
        let filename = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| InstallError::Setup(format!("invalid theme path: {path}")))?
            .to_owned();
        if !filenames.insert(filename.clone()) {
            return Err(InstallError::Setup(format!(
                "duplicate theme filename: {filename}"
            )));
        }
        let url = format!("{}/{}", raw_url.trim_end_matches('/'), path);
        if files.len() >= MAX_THEME_FILES {
            return Err(InstallError::Setup(format!(
                "Catppuccin theme count exceeds {}",
                MAX_THEME_FILES
            )));
        }
        let bytes = context.http.get_bytes(&url)?;
        if bytes.len() > MAX_THEME_FILE_BYTES {
            return Err(InstallError::Setup(format!(
                "Catppuccin theme {path} exceeds the {}-byte limit",
                MAX_THEME_FILE_BYTES
            )));
        }
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| InstallError::Setup("Catppuccin theme byte count overflowed".into()))?;
        if total_bytes > MAX_THEME_TOTAL_BYTES {
            return Err(InstallError::Setup(format!(
                "Catppuccin themes exceed the {}-byte total limit",
                MAX_THEME_TOTAL_BYTES
            )));
        }
        serde_json::from_slice::<Value>(&bytes).map_err(|error| {
            InstallError::Setup(format!("invalid Catppuccin theme {path}: {error}"))
        })?;
        files.push(ThemeFile { filename, bytes });
    }
    if files.is_empty() {
        return Err(InstallError::Setup(
            "no Catppuccin OpenCode themes discovered".into(),
        ));
    }
    Ok(ThemeTree { files })
}

fn stage_themes(root: &Path, themes: &ThemeTree) -> Result<Vec<(String, Vec<u8>)>> {
    let theme_root = root.join("themes");
    fs::create_dir_all(&theme_root)?;
    let mut staged = Vec::new();
    for theme in &themes.files {
        let path = theme_root.join(&theme.filename);
        fs::write(&path, &theme.bytes)?;
        serde_json::from_slice::<Value>(&fs::read(&path)?).map_err(|error| {
            InstallError::Setup(format!(
                "staged theme {} is invalid: {error}",
                theme.filename
            ))
        })?;
        staged.push((theme.filename.clone(), fs::read(path)?));
    }
    Ok(staged)
}

fn opencode_config_dir(platform: &dyn PlatformOps) -> Result<PathBuf> {
    let home = platform.home_dir()?;
    let root = if platform.platform() == Platform::Linux {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
    } else {
        home.join(".config")
    };
    Ok(root.join("opencode"))
}

fn config_path(platform: &dyn PlatformOps, directory: &Path) -> PathBuf {
    directory.join(config_filename(platform))
}

fn config_filename(platform: &dyn PlatformOps) -> &'static str {
    if platform.platform() == Platform::Windows {
        "opencode.jsonc"
    } else {
        "opencode.json"
    }
}
