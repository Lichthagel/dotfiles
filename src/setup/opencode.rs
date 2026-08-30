use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{InstallError, Result};
use crate::model::Platform;
use crate::platform::PlatformOps;
use crate::secrets::SecretValues;
use zeroize::Zeroizing;

use super::http::MAX_THEME_FILE_BYTES;
use super::jsonc::JsoncDocument;
use super::SetupContext;

pub const SUPERPOWERS_PLUGIN: &str = "superpowers@git+https://github.com/obra/superpowers.git";
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
        .replace_file(&config_file, &fs::read(&staged_config)?)?;
    for (filename, bytes) in staged_credentials {
        context
            .platform
            .replace_private_file(&config_dir.join(filename), &bytes)?;
    }
    let themes_dir = config_dir.join("themes");
    fs::create_dir_all(&themes_dir)?;
    for (filename, bytes) in staged_themes {
        context
            .platform
            .replace_file(&themes_dir.join(filename), &bytes)?;
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

    if configure_providers {
        let provider = object
            .entry("provider".to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        let provider = provider.as_object_mut().ok_or_else(|| {
            InstallError::Setup("OpenCode provider configuration must be an object".into())
        })?;
        provider.insert(
            "openrouter".into(),
            serde_json::json!({
                "options": {"apiKey": format!("{{file:{}}}", config_dir.join("openrouter-api-key").display())}
            }),
        );
        provider.insert(
            "azure".into(),
            serde_json::json!({
                "options": {
                    "apiKey": format!("{{file:{}}}", config_dir.join("azure-api-key").display()),
                    "resourceName": format!("{{file:{}}}", config_dir.join("azure-resource-name").display())
                }
            }),
        );
        provider.insert(
            "digitalocean".into(),
            serde_json::json!({
                "options": {"apiKey": format!("{{file:{}}}", config_dir.join("digitalocean-access-token").display())}
            }),
        );
    }

    let mcp = object
        .entry("mcp".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp = mcp.as_object_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode mcp configuration must be an object".into())
    })?;
    mcp.insert(
        "ddg-search".into(),
        serde_json::json!({
            "type": "local",
            "command": ["uvx", "duckduckgo-mcp-server"],
            "environment": {"DDG_SAFE_SEARCH": "OFF"}
        }),
    );

    let plugins = object
        .entry("plugin".to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    let plugins = plugins.as_array_mut().ok_or_else(|| {
        InstallError::Setup("OpenCode plugin configuration must be an array".into())
    })?;
    plugins.retain(|plugin| plugin.as_str() != Some(SUPERPOWERS_PLUGIN));
    plugins.push(Value::String(SUPERPOWERS_PLUGIN.to_owned()));
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
