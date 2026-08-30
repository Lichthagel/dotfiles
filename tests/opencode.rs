mod support;

use std::fs;

use dotfiles::error::InstallError;
use dotfiles::model::Platform;
use dotfiles::setup::http::MAX_THEME_FILE_BYTES;
use dotfiles::setup::opencode::{configure_opencode, SUPERPOWERS_PLUGIN};
use support::opencode::{config_path_is_private, OpenCodeFixture};

#[test]
fn opencode_is_idempotent_and_does_not_write_secret_values_to_config() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
    configure_opencode(&fixture.context()).unwrap();
    let first = fixture.read_config();
    configure_opencode(&fixture.context()).unwrap();
    let second = fixture.read_config();

    assert_eq!(first, second);
    assert_eq!(fixture.plugin_count(SUPERPOWERS_PLUGIN), 1);
    assert_eq!(fixture.value_count_in_config("openrouter-test-secret"), 0);
    let config = fixture.read_config();
    assert_eq!(
        config["provider"]["openrouter"]["options"]["apiKey"],
        format!(
            "{{file:{}}}",
            fixture.config_dir().join("openrouter-api-key").display()
        )
    );
    assert_eq!(
        config["provider"]["azure"]["options"]["apiKey"],
        format!(
            "{{file:{}}}",
            fixture.config_dir().join("azure-api-key").display()
        )
    );
    assert_eq!(
        config["provider"]["azure"]["options"]["resourceName"],
        format!(
            "{{file:{}}}",
            fixture.config_dir().join("azure-resource-name").display()
        )
    );
    assert_eq!(
        config["provider"]["digitalocean"]["options"]["apiKey"],
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
        fixture.read_config()["mcp"]["ddg-search"]["command"],
        serde_json::json!(["uvx", "duckduckgo-mcp-server"])
    );
    assert!(!fixture.read_config().get("theme").is_some());
}

#[test]
fn themes_are_flattened_and_unrelated_themes_are_preserved() {
    let fixture = OpenCodeFixture::with_jsonc_and_existing_plugin();
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

#[test]
fn explicit_platform_fixtures_use_platform_config_filenames() {
    for platform in [Platform::Linux, Platform::Windows] {
        let fixture = OpenCodeFixture::for_platform(platform);
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
    let before = fixture.snapshot_managed_outputs();
    let error = configure_opencode(&fixture.context()).unwrap_err();
    assert!(matches!(error, InstallError::Secret(_)));
    assert_eq!(fixture.snapshot_managed_outputs(), before);
    assert!(!fixture.config_dir().exists());
}

#[test]
fn malformed_theme_does_not_replace_config_credentials_or_themes() {
    let fixture = OpenCodeFixture::with_malformed_theme_client();
    let before = fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&fixture.context()).is_err());
    assert_eq!(fixture.snapshot_managed_outputs(), before);
}

#[test]
fn failed_theme_download_does_not_replace_config_credentials_or_themes() {
    let fixture = OpenCodeFixture::with_failing_theme_client();
    let before = fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&fixture.context()).is_err());
    assert_eq!(fixture.snapshot_managed_outputs(), before);
}

#[test]
fn theme_count_and_file_size_limits_fail_before_replacement() {
    let count_fixture = OpenCodeFixture::with_http(support::opencode::FakeHttp::too_many_themes());
    let count_before = count_fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&count_fixture.context()).is_err());
    assert_eq!(count_fixture.snapshot_managed_outputs(), count_before);

    let size_fixture = OpenCodeFixture::with_http(support::opencode::FakeHttp::oversized_theme());
    let size_before = size_fixture.snapshot_managed_outputs();
    assert!(configure_opencode(&size_fixture.context()).is_err());
    assert_eq!(size_fixture.snapshot_managed_outputs(), size_before);
    assert!(MAX_THEME_FILE_BYTES > 0);
}
