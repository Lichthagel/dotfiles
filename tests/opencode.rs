mod support;

use std::fs;

use dotfiles::error::InstallError;
use dotfiles::model::Platform;
use dotfiles::modules::opencode::{configure_opencode, SUPERPOWERS_PLUGIN};
use dotfiles::setup::http::MAX_THEME_FILE_BYTES;
use support::opencode::{config_path_is_private, OpenCodeFixture};

#[test]
fn opencode_is_idempotent_and_does_not_write_secret_values_to_config() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    configure_opencode(&fixture.context()).unwrap();
    let first = fixture.read_config();
    let paths = [
        fixture.config_file(),
        fixture.themes_dir().join("catppuccin-frappe.json"),
        fixture.themes_dir().join("catppuccin-mocha.json"),
        fixture.config_dir().join("openrouter-api-key"),
        fixture.config_dir().join("azure-api-key"),
        fixture.config_dir().join("azure-resource-name"),
        fixture.config_dir().join("digitalocean-access-token"),
    ];
    let modified_before = paths
        .iter()
        .map(|path| fs::metadata(path).unwrap().modified().unwrap())
        .collect::<Vec<_>>();
    let backup_count = count_files(&fixture.ops.backup_root().unwrap());
    configure_opencode(&fixture.context()).unwrap();
    let second = fixture.read_config();

    assert_eq!(first, second);
    assert_eq!(
        paths
            .iter()
            .map(|path| fs::metadata(path).unwrap().modified().unwrap())
            .collect::<Vec<_>>(),
        modified_before
    );
    assert_eq!(
        count_files(&fixture.ops.backup_root().unwrap()),
        backup_count
    );
    assert_eq!(fixture.plugin_count(SUPERPOWERS_PLUGIN), 1);
    assert_eq!(fixture.value_count_in_config("openrouter-test-secret"), 0);
    let config = fixture.read_config();
    assert_eq!(
        config["providers"]["openrouter"]["settings"]["apiKey"],
        format!(
            "{{file:{}}}",
            fixture.config_dir().join("openrouter-api-key").display()
        )
    );
    assert_eq!(
        config["providers"]["azure"]["settings"]["apiKey"],
        format!(
            "{{file:{}}}",
            fixture.config_dir().join("azure-api-key").display()
        )
    );
    assert_eq!(
        config["providers"]["azure"]["settings"]["resourceName"],
        format!(
            "{{file:{}}}",
            fixture.config_dir().join("azure-resource-name").display()
        )
    );
    assert_eq!(
        config["providers"]["digitalocean"]["settings"]["apiKey"],
        format!(
            "{{file:{}}}",
            fixture
                .config_dir()
                .join("digitalocean-access-token")
                .display()
        )
    );
    assert_eq!(config["unrelated"]["keep"], true);
    assert_eq!(
        fixture.read_config()["mcp"]["servers"]["ddg-search"]["command"],
        serde_json::json!(["uvx", "duckduckgo-mcp-server"])
    );
    let config = fixture.read_config();
    assert_eq!(config["$schema"], "https://opencode.ai/config.json");
    assert_eq!(config["plugins"].as_array().unwrap().len(), 2);
    assert!(config.get("plugin").is_none());
    assert!(config.get("theme").is_none());
}

#[test]
fn legacy_opencode_settings_are_migrated_to_v2_shapes() {
    let fixture = OpenCodeFixture::for_platform_with_secrets(Platform::Linux);
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    fs::create_dir_all(fixture.config_dir()).unwrap();
    fs::write(
        fixture.config_file(),
        r#"{
          "provider": {
            "custom": {"npm": "@example/provider", "options": {"baseURL": "https://api.example.test"}},
            "openrouter": {"options": {"legacySetting": true}}
          },
          "mcp": {
            "legacy-search": {"type": "local", "command": ["legacy-mcp"]},
            "timeout": {"startup": 1000}
          },
          "plugin": ["legacy-plugin", ["tuple-plugin", {"enabled": true}]]
        }"#,
    )
    .unwrap();

    configure_opencode(&fixture.context()).unwrap();
    let config = fixture.read_config();

    assert!(config.get("provider").is_none());
    assert_eq!(
        config["providers"]["custom"]["package"],
        "@example/provider"
    );
    assert_eq!(
        config["providers"]["custom"]["settings"]["baseURL"],
        "https://api.example.test"
    );
    assert_eq!(
        config["providers"]["openrouter"]["settings"]["legacySetting"],
        true
    );
    assert_eq!(
        config["mcp"]["servers"]["legacy-search"]["command"],
        serde_json::json!(["legacy-mcp"])
    );
    assert_eq!(config["mcp"]["timeout"]["startup"], 1000);
    assert_eq!(config["plugins"][0], "legacy-plugin");
    assert_eq!(
        config["plugins"][1],
        serde_json::json!({"package": "tuple-plugin", "options": {"enabled": true}})
    );
    assert!(config.get("plugin").is_none());
}

fn count_files(root: &std::path::Path) -> usize {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            if entry.path().is_dir() {
                count_files(&entry.path())
            } else {
                1
            }
        })
        .sum()
}

#[test]
fn changed_theme_updates_only_its_managed_output() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    configure_opencode(&fixture.context()).unwrap();

    let config_path = fixture.config_file();
    let mocha_path = fixture.themes_dir().join("catppuccin-mocha.json");
    let config_modified = fs::metadata(&config_path).unwrap().modified().unwrap();
    let mocha_modified = fs::metadata(&mocha_path).unwrap().modified().unwrap();
    let backups_before = count_files(&fixture.ops.backup_root().unwrap());
    fixture
        .http
        .set_bytes("frappe", br#"{"name":"updated-frappe"}"#.to_vec());
    configure_opencode(&fixture.context()).unwrap();

    assert_eq!(
        fs::read(fixture.themes_dir().join("catppuccin-frappe.json")).unwrap(),
        br#"{"name":"updated-frappe"}"#
    );
    assert_eq!(
        fs::metadata(&config_path).unwrap().modified().unwrap(),
        config_modified
    );
    assert_eq!(
        fs::metadata(&mocha_path).unwrap().modified().unwrap(),
        mocha_modified
    );
    assert_eq!(
        count_files(&fixture.ops.backup_root().unwrap()),
        backups_before + 1
    );
}

#[test]
fn themes_are_flattened_and_unrelated_themes_are_preserved() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    configure_opencode(&fixture.context()).unwrap();

    assert_eq!(
        fs::read(fixture.themes_dir().join("catppuccin-frappe.json")).unwrap(),
        br#"{"name":"frappe"}"#
    );
    assert_eq!(
        fs::read(fixture.themes_dir().join("catppuccin-mocha.json")).unwrap(),
        br#"{"name":"mocha"}"#
    );
    assert!(fixture.themes_dir().join("catppuccin-old.json").exists());
    assert!(fixture.themes_dir().join("unrelated.json").exists());
    assert!(fixture
        .themes_dir()
        .join("local/catppuccin-local.json")
        .exists());
}

#[test]
fn credentials_are_written_protected_and_never_to_config() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    configure_opencode(&fixture.context()).unwrap();

    for (name, value) in [
        ("openrouter-api-key", "openrouter-test-secret"),
        ("azure-api-key", "azure-test-secret"),
        ("azure-resource-name", "azure-resource-test"),
        ("digitalocean-access-token", "digitalocean-test-secret"),
    ] {
        let path = fixture.config_dir().join(name);
        assert_eq!(fs::read_to_string(&path).unwrap(), value);
        assert!(
            config_path_is_private(&path),
            "credential is not private: {name}"
        );
    }
}

#[cfg(unix)]
#[test]
fn matching_credentials_with_unsafe_modes_are_secured_without_backup() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    configure_opencode(&fixture.context()).unwrap();
    let credential = fixture.config_dir().join("openrouter-api-key");
    let modified = fs::metadata(&credential).unwrap().modified().unwrap();
    let backups = count_files(&fixture.ops.backup_root().unwrap());
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o644)).unwrap();

    configure_opencode(&fixture.context()).unwrap();

    assert_eq!(fs::read(&credential).unwrap(), b"openrouter-test-secret");
    assert_eq!(
        fs::metadata(&credential).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&credential).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(count_files(&fixture.ops.backup_root().unwrap()), backups);
}

#[test]
fn explicit_platform_fixtures_use_platform_config_filenames() {
    for platform in [Platform::Linux, Platform::Windows] {
        let fixture = OpenCodeFixture::for_platform(platform);
        let _config_home =
            support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
        configure_opencode(&fixture.context()).unwrap();
        let expected = if platform == Platform::Windows {
            "opencode.jsonc"
        } else {
            "opencode.json"
        };
        assert!(fixture.config_dir().join(expected).exists());
        assert!(!fixture
            .config_dir()
            .join(if platform == Platform::Windows {
                "opencode.json"
            } else {
                "opencode.jsonc"
            })
            .exists());
    }
}

#[cfg(windows)]
#[test]
fn windows_fixture_inspects_current_user_dacl() {
    let fixture = OpenCodeFixture::for_platform_with_secrets(Platform::Windows);
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    configure_opencode(&fixture.context()).unwrap();
    for name in [
        "openrouter-api-key",
        "azure-api-key",
        "azure-resource-name",
        "digitalocean-access-token",
    ] {
        assert!(config_path_is_private(&fixture.config_dir().join(name)));
    }
}

#[test]
fn no_provider_secrets_leave_existing_provider_configuration_unchanged() {
    let fixture = OpenCodeFixture::with_no_secrets();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    let before = fs::read(fixture.config_file()).unwrap();
    configure_opencode(&fixture.context()).unwrap();
    let config = fixture.read_config();
    assert_eq!(config["provider"]["legacy"]["keep"], true);
    assert_ne!(fs::read(fixture.config_file()).unwrap(), before);
    for name in [
        "openrouter-api-key",
        "azure-api-key",
        "azure-resource-name",
        "digitalocean-access-token",
    ] {
        assert!(!fixture.config_dir().join(name).exists());
    }
}

#[test]
fn partial_provider_secrets_fail_before_any_output_changes() {
    let fixture = OpenCodeFixture::with_partial_secrets();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    let before = fixture.snapshot_managed_outputs();
    let error = configure_opencode(&fixture.context()).unwrap_err();
    assert!(matches!(error, InstallError::Secret(_)));
    assert_eq!(fixture.snapshot_managed_outputs(), before);
    assert!(!fixture.config_dir().exists());
}

#[test]
fn malformed_theme_does_not_replace_config_credentials_or_themes() {
    let fixture = OpenCodeFixture::with_malformed_theme_client();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    let before = fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&fixture.context()).is_err());
    assert_eq!(fixture.snapshot_managed_outputs(), before);
}

#[test]
fn failed_theme_download_does_not_replace_config_credentials_or_themes() {
    let fixture = OpenCodeFixture::with_failing_theme_client();
    let _config_home =
        support::ScopedXdgConfigHome::new(fixture.config_dir().parent().expect("config home"));
    let before = fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&fixture.context()).is_err());
    assert_eq!(fixture.snapshot_managed_outputs(), before);
}

#[test]
fn theme_count_and_file_size_limits_fail_before_replacement() {
    let count_fixture = OpenCodeFixture::with_http(support::opencode::FakeHttp::too_many_themes());
    let count_before = count_fixture.snapshot_managed_outputs();
    {
        let _config_home = support::ScopedXdgConfigHome::new(
            count_fixture.config_dir().parent().expect("config home"),
        );
        assert!(configure_opencode(&count_fixture.context()).is_err());
    }
    assert_eq!(count_fixture.snapshot_managed_outputs(), count_before);

    let size_fixture = OpenCodeFixture::with_http(support::opencode::FakeHttp::oversized_theme());
    let size_before = size_fixture.snapshot_managed_outputs();
    {
        let _config_home = support::ScopedXdgConfigHome::new(
            size_fixture.config_dir().parent().expect("config home"),
        );
        assert!(configure_opencode(&size_fixture.context()).is_err());
    }
    assert_eq!(size_fixture.snapshot_managed_outputs(), size_before);
    assert!(MAX_THEME_FILE_BYTES > 0);
}
