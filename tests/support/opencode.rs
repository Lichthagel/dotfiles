use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dotfiles::error::Result;
use dotfiles::model::Platform;
use dotfiles::platform::{PathContext, PlatformOps};
use dotfiles::secrets::SecretValues;
use dotfiles::setup::{HttpClient, SetupContext};
use tempfile::TempDir;

#[derive(Clone, Default)]
pub struct FakeHttp {
    json: Arc<Mutex<Option<serde_json::Value>>>,
    bytes: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    fail: Arc<Mutex<Option<String>>>,
}

impl FakeHttp {
    pub fn themes() -> Self {
        let client = Self::default();
        client.set_json(serde_json::json!({
            "tree": [
                {"path": "themes/frappe/catppuccin-frappe.json", "type": "blob"},
                {"path": "themes/mocha/catppuccin-mocha.json", "type": "blob"},
                {"path": "README.md", "type": "blob"}
            ]
        }));
        client.set_bytes("frappe", br#"{"name":"frappe"}"#.to_vec());
        client.set_bytes("mocha", br#"{"name":"mocha"}"#.to_vec());
        client
    }

    pub fn failing() -> Self {
        let client = Self::themes();
        *client.fail.lock().unwrap() = Some("theme download failed".into());
        client
    }

    fn set_json(&self, value: serde_json::Value) {
        *self.json.lock().unwrap() = Some(value);
    }

    fn set_bytes(&self, key: &str, value: Vec<u8>) {
        self.bytes.lock().unwrap().insert(key.into(), value);
    }
}

impl HttpClient for FakeHttp {
    fn get_json(&self, _url: &str) -> Result<serde_json::Value> {
        if let Some(message) = self.fail.lock().unwrap().clone() {
            return Err(dotfiles::error::InstallError::Setup(message));
        }
        self.json
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| dotfiles::error::InstallError::Setup("missing fake theme tree".into()))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        if let Some(message) = self.fail.lock().unwrap().clone() {
            return Err(dotfiles::error::InstallError::Setup(message));
        }
        let key = if url.contains("frappe") {
            "frappe"
        } else {
            "mocha"
        };
        self.bytes
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| dotfiles::error::InstallError::Setup("missing fake theme".into()))
    }
}

pub struct OpenCodeFixture {
    pub temp: TempDir,
    pub ops: Box<dyn PlatformOps>,
    pub http: FakeHttp,
    pub secrets: Option<SecretValues>,
}

impl OpenCodeFixture {
    pub fn with_jsonc_and_existing_plugin() -> Self {
        let fixture = Self::new(FakeHttp::themes(), Some(all_secrets()));
        fs::create_dir_all(fixture.config_dir()).unwrap();
        let config = r#"{
          // preserve this comment while reading JSONC
          "unrelated": {"keep": true,},
          "plugin": ["local-plugin", "superpowers@git+https://github.com/obra/superpowers.git", "superpowers@git+https://github.com/obra/superpowers.git"],
        }"#;
        fs::write(fixture.config_file(), config).unwrap();
        fixture.add_existing_themes();
        fixture
    }

    pub fn with_failing_theme_client() -> Self {
        let fixture = Self::new(FakeHttp::failing(), Some(all_secrets()));
        fs::create_dir_all(fixture.config_dir()).unwrap();
        fs::write(fixture.config_file(), r#"{"unrelated":{"keep":true}}"#).unwrap();
        fixture.add_existing_themes();
        fixture
    }

    pub fn with_no_secrets() -> Self {
        let fixture = Self::new(FakeHttp::themes(), None);
        fs::create_dir_all(fixture.config_dir()).unwrap();
        fs::write(
            fixture.config_file(),
            r#"{"provider":{"legacy":{"keep":true}}}"#,
        )
        .unwrap();
        fixture.add_existing_themes();
        fixture
    }

    pub fn with_partial_secrets() -> Self {
        let fixture = Self::new(
            FakeHttp::themes(),
            Some(
                SecretValues::from_pairs([("OPENROUTER_API_KEY", "openrouter-test-secret")])
                    .unwrap(),
            ),
        );
        fs::create_dir_all(fixture.config_dir()).unwrap();
        fs::write(fixture.config_file(), r#"{"unrelated":{"keep":true}}"#).unwrap();
        fixture.add_existing_themes();
        fixture
    }

    fn new(http: FakeHttp, secrets: Option<SecretValues>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let profile = if cfg!(windows) {
            Some(home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"))
        } else {
            None
        };
        let context = PathContext::new(home, profile, temp.path().join("state/dotfiles/backups"));
        let ops: Box<dyn PlatformOps> = platform_ops(context);
        Self {
            temp,
            ops,
            http,
            secrets,
        }
    }

    pub fn context(&self) -> SetupContext<'_> {
        SetupContext {
            platform: self.ops.as_ref(),
            runner: &NoopRunner,
            http: &self.http,
            secrets: self.secrets.as_ref(),
        }
    }

    pub fn config_dir(&self) -> PathBuf {
        self.ops.home_dir().unwrap().join(".config/opencode")
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join(if cfg!(windows) {
            "opencode.jsonc"
        } else {
            "opencode.json"
        })
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.config_dir().join("themes")
    }

    pub fn add_existing_themes(&self) {
        fs::create_dir_all(self.themes_dir().join("local")).unwrap();
        fs::write(
            self.themes_dir().join("catppuccin-old.json"),
            br#"{"name":"old"}"#,
        )
        .unwrap();
        fs::write(
            self.themes_dir().join("unrelated.json"),
            br#"{"name":"keep"}"#,
        )
        .unwrap();
        fs::write(
            self.themes_dir().join("local/catppuccin-local.json"),
            br#"{"name":"local"}"#,
        )
        .unwrap();
    }

    pub fn read_config(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.config_file()).unwrap()).unwrap()
    }

    pub fn plugin_count(&self, name: &str) -> usize {
        self.read_config()
            .get("plugin")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter(|value| value.as_str() == Some(name))
            .count()
    }

    pub fn value_count_in_config(&self, value: &str) -> usize {
        fn count(value: &serde_json::Value, needle: &str) -> usize {
            match value {
                serde_json::Value::String(text) => usize::from(text == needle),
                serde_json::Value::Array(values) => {
                    values.iter().map(|value| count(value, needle)).sum()
                }
                serde_json::Value::Object(values) => {
                    values.values().map(|value| count(value, needle)).sum()
                }
                _ => 0,
            }
        }
        count(&self.read_config(), value)
    }

    pub fn snapshot_managed_outputs(&self) -> BTreeMap<String, Vec<u8>> {
        let mut snapshot = BTreeMap::new();
        for path in [
            self.config_file(),
            self.themes_dir().join("catppuccin-frappe.json"),
            self.themes_dir().join("catppuccin-mocha.json"),
        ] {
            if path.exists() {
                snapshot.insert(path.to_string_lossy().into_owned(), fs::read(path).unwrap());
            }
        }
        for name in [
            "openrouter-api-key",
            "azure-api-key",
            "azure-resource-name",
            "digitalocean-access-token",
        ] {
            let path = self.config_dir().join(name);
            if path.exists() {
                snapshot.insert(path.to_string_lossy().into_owned(), fs::read(path).unwrap());
            }
        }
        snapshot
    }
}

fn all_secrets() -> SecretValues {
    SecretValues::from_pairs([
        ("OPENROUTER_API_KEY", "openrouter-test-secret"),
        ("AZURE_API_KEY", "azure-test-secret"),
        ("AZURE_RESOURCE_NAME", "azure-resource-test"),
        ("DIGITALOCEAN_ACCESS_TOKEN", "digitalocean-test-secret"),
    ])
    .unwrap()
}

#[derive(Clone, Copy)]
struct NoopRunner;

impl dotfiles::packages::CommandRunner for NoopRunner {
    fn available(&self, _program: &str) -> bool {
        false
    }
    fn run(
        &self,
        _program: &str,
        _args: &[String],
        _env: &[(String, String)],
        _stdin: Option<&[u8]>,
    ) -> Result<dotfiles::packages::CommandOutput> {
        Ok(dotfiles::packages::CommandOutput {
            status: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn platform_ops(context: PathContext) -> Box<dyn PlatformOps> {
    #[cfg(unix)]
    {
        Box::new(dotfiles::platform::unix::UnixPlatform::new(context))
    }
    #[cfg(windows)]
    {
        Box::new(dotfiles::platform::windows::WindowsPlatform::new(context))
    }
}

pub fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

pub fn config_path_is_private(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return fs::metadata(path).unwrap().permissions().mode() & 0o777 == 0o600;
    }
    #[cfg(windows)]
    {
        let _ = path;
        return true;
    }
}
