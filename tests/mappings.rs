mod support;

use dotfiles::assets::asset_bytes;
use dotfiles::mappings::{apply_mapping, backup_root, resolve_target};
use dotfiles::model::{Platform, Target};
use dotfiles::platform::{PlatformOps, SystemTimestamp};
use dotfiles::registry::{self, MappingBehavior, MappingSpec};
use std::fs;
use std::sync::{Mutex, OnceLock};
use support::mappings::{
    current_platform, git_mapping, mapping_target, profile_target, MappingFixture,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn existing_target_is_backed_up_and_embedded_asset_is_a_regular_file() {
    let fixture = MappingFixture::new();
    fixture.write_target(".gitconfig", b"old");
    apply_mapping(&fixture.ops, git_mapping(), b"new", &SystemTimestamp).unwrap();

    assert_eq!(fixture.read_target(".gitconfig"), b"new");
    assert!(!fixture.target(".gitconfig").is_symlink());
    assert_eq!(fixture.read_backup(".gitconfig"), b"old");
}

#[test]
fn first_install_materializes_the_embedded_asset() {
    let fixture = MappingFixture::new();
    let mapping = git_mapping();
    apply_mapping(
        &fixture.ops,
        mapping,
        asset_bytes(mapping.asset),
        &SystemTimestamp,
    )
    .unwrap();

    assert_eq!(
        fixture.read_target(".gitconfig"),
        asset_bytes(mapping.asset)
    );
    assert!(!fixture.target(".gitconfig").is_symlink());
}

#[test]
fn failed_write_restores_the_previous_target() {
    let fixture = MappingFixture::with_failing_rename();
    fixture.write_target(".gitconfig", b"old");
    assert!(apply_mapping(&fixture.ops, git_mapping(), b"new", &SystemTimestamp).is_err());
    assert_eq!(fixture.read_target(".gitconfig"), b"old");
}

#[test]
fn existing_directory_is_backed_up_before_file_replacement() {
    let fixture = MappingFixture::new();
    let target = fixture.target(".gitconfig");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("old"), b"old").unwrap();

    apply_mapping(&fixture.ops, git_mapping(), b"new", &SystemTimestamp).unwrap();

    assert_eq!(fixture.read_target(".gitconfig"), b"new");
    assert_eq!(fixture.read_backup(".gitconfig/old"), b"old");
}

#[cfg(unix)]
#[test]
fn existing_symlink_is_backed_up_without_following_it() {
    use std::os::unix::fs::symlink;

    let fixture = MappingFixture::new();
    let source = fixture.temp.path().join("source");
    fs::write(&source, b"linked").unwrap();
    let target = fixture.target(".gitconfig");
    symlink(&source, &target).unwrap();

    apply_mapping(&fixture.ops, git_mapping(), b"new", &SystemTimestamp).unwrap();

    assert_eq!(fixture.read_target(".gitconfig"), b"new");
    assert!(!target.is_symlink());
    assert_eq!(fixture.read_backup(".gitconfig"), b"linked");
}

#[test]
fn profile_targets_are_resolved_only_on_windows() {
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let profile = fixture
        .ops
        .context()
        .profile
        .as_ref()
        .expect("Windows profile");
    assert_eq!(
        resolve_target(&fixture.ops, Target::ProfileFile).unwrap(),
        profile.clone()
    );
    assert_eq!(
        profile_target(&fixture.ops, Target::ProfileSibling("Profile.d/test.ps1")),
        profile.parent().unwrap().join("Profile.d/test.ps1")
    );
}

#[test]
fn linux_profile_target_is_rejected() {
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let error = resolve_target(&fixture.ops, Target::ProfileFile).unwrap_err();
    assert!(error.to_string().contains("PowerShell profile"));
}

#[test]
fn parent_directory_targets_are_rejected() {
    let fixture = MappingFixture::new();
    let error = resolve_target(&fixture.ops, Target::Home("../outside")).unwrap_err();
    assert!(error.to_string().contains("must stay below"));
}

#[cfg(windows)]
#[test]
fn windows_root_relative_targets_are_rejected() {
    let fixture = MappingFixture::with_platform(Platform::Windows);
    let error = resolve_target(&fixture.ops, Target::Home(r"\outside")).unwrap_err();
    assert!(error.to_string().contains("must stay below"));
}

#[test]
fn backup_setup_failure_removes_staged_file() {
    let fixture = MappingFixture::new();
    fixture.write_target(".gitconfig", b"old");
    fs::create_dir_all(fixture.ops.context().backup_root.parent().unwrap()).unwrap();
    fs::write(&fixture.ops.context().backup_root, b"not-a-directory").unwrap();

    assert!(apply_mapping(&fixture.ops, git_mapping(), b"new", &SystemTimestamp).is_err());
    let staged = fs::read_dir(fixture.ops.context().home.clone())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".dotfiles-")
        })
        .count();
    assert_eq!(staged, 0);
    assert_eq!(fixture.read_target(".gitconfig"), b"old");
}

#[cfg(unix)]
#[test]
fn private_file_replacement_sets_mode_0600() {
    use dotfiles::platform::unix::UnixPlatform;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let platform = UnixPlatform::new(dotfiles::platform::PathContext::new(
        home.clone(),
        None,
        temp.path().join("state"),
    ));
    let target = home.join("credential");
    platform.replace_private_file(&target, b"secret").unwrap();
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn private_file_backup_is_protected_before_replacement() {
    use dotfiles::platform::unix::UnixPlatform;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let backup_root = temp.path().join("state/dotfiles/backups");
    let platform = UnixPlatform::new(dotfiles::platform::PathContext::new(
        home.clone(),
        None,
        backup_root.clone(),
    ));
    let target = home.join("credential");
    fs::write(&target, b"old").unwrap();
    platform.replace_private_file(&target, b"new").unwrap();
    let timestamp = fs::read_dir(&backup_root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let backup = timestamp.join("credential");
    assert_eq!(
        fs::metadata(backup).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[cfg(windows)]
#[test]
fn private_file_replacement_grants_only_current_user_full_control() {
    use dotfiles::platform::windows::WindowsPlatform;
    use std::process::Command;
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let platform = WindowsPlatform::new(dotfiles::platform::PathContext::new(
        home.clone(),
        Some(home.join("profile.ps1")),
        temp.path().join("state"),
    ));
    let target = home.join("credential");
    platform.replace_private_file(&target, b"secret").unwrap();
    let whoami = String::from_utf8(Command::new("whoami").output().unwrap().stdout).unwrap();
    let account = whoami.trim().rsplit('\\').next().unwrap();
    let acl =
        String::from_utf8(Command::new("icacls").arg(&target).output().unwrap().stdout).unwrap();
    let entries: Vec<_> = acl.lines().filter(|line| line.contains(":(")).collect();
    assert_eq!(entries.len(), 1, "unexpected ACL entries: {acl}");
    assert!(entries[0].contains(account));
    assert!(entries[0].contains("(F)"));
}

#[test]
fn backup_root_comes_from_the_platform_context() {
    let fixture = MappingFixture::new();
    assert_eq!(
        backup_root(&fixture.ops).unwrap(),
        fixture.ops.context().backup_root
    );
}

#[test]
fn current_registry_mapping_resolves_under_home() {
    let fixture = MappingFixture::new();
    let target = mapping_target(&fixture.ops, git_mapping());
    assert_eq!(target, fixture.ops.context().home.join(".gitconfig"));
    assert_eq!(current_platform(), fixture.ops.platform());
}

#[cfg(unix)]
#[test]
fn environment_home_and_state_overrides_are_used_by_unix_platform() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("override-home");
    let state = root.path().join("override-state");
    let old_home = std::env::var_os("DOTFILES_HOME");
    let old_state = std::env::var_os("XDG_STATE_HOME");
    std::env::set_var("DOTFILES_HOME", &home);
    std::env::set_var("XDG_STATE_HOME", &state);

    let platform = dotfiles::platform::unix::from_environment().unwrap();
    assert_eq!(platform.home_dir().unwrap(), home);
    assert_eq!(
        platform.backup_root().unwrap(),
        state.join("dotfiles/backups")
    );
    assert_eq!(
        platform.state_path().unwrap(),
        state.join("dotfiles/state.json")
    );

    restore_env("DOTFILES_HOME", old_home);
    restore_env("XDG_STATE_HOME", old_state);
}

#[cfg(windows)]
#[test]
fn environment_profile_and_localappdata_overrides_are_used_by_windows_platform() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("override-home");
    let profile = root.path().join("profile.ps1");
    let local = root.path().join("override-local");
    let old_home = std::env::var_os("DOTFILES_HOME");
    let old_profile = std::env::var_os("DOTFILES_PROFILE");
    let old_local = std::env::var_os("LOCALAPPDATA");
    std::env::set_var("DOTFILES_HOME", &home);
    std::env::set_var("DOTFILES_PROFILE", &profile);
    std::env::set_var("LOCALAPPDATA", &local);

    let platform = dotfiles::platform::windows::from_environment().unwrap();
    assert_eq!(platform.home_dir().unwrap(), home);
    assert_eq!(platform.profile_path().unwrap(), profile);
    assert_eq!(
        platform.backup_root().unwrap(),
        local.join("dotfiles/backups")
    );
    assert_eq!(
        platform.state_path().unwrap(),
        local.join("dotfiles/state.json")
    );

    restore_env("DOTFILES_HOME", old_home);
    restore_env("DOTFILES_PROFILE", old_profile);
    restore_env("LOCALAPPDATA", old_local);
}

#[test]
fn shell_profile_mappings_preserve_unmanaged_content_and_update_only_the_managed_section() {
    let cases = [
        (
            "bash",
            Platform::Linux,
            "# >>> dotfiles managed bash section >>>",
            "# <<< dotfiles managed bash section <<<",
        ),
        (
            "zsh",
            Platform::Linux,
            "# >>> dotfiles managed zsh section >>>",
            "# <<< dotfiles managed zsh section <<<",
        ),
        (
            "powershell",
            Platform::Windows,
            "# >>> dotfiles managed powershell section >>>",
            "# <<< dotfiles managed powershell section <<<",
        ),
    ];

    for (module_id, platform, start, end) in cases {
        let fixture = MappingFixture::with_platform(platform);
        let mapping = shell_mapping(module_id, platform);
        assert!(matches!(
            mapping.behavior,
            MappingBehavior::ManagedSection { .. }
        ));
        let target = fixture.ops.resolve_target(mapping.target).unwrap();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let user_content = b"# user configuration without a final newline";
        fs::write(&target, user_content).unwrap();

        apply_mapping(
            &fixture.ops,
            mapping,
            asset_bytes(mapping.asset),
            &SystemTimestamp,
        )
        .unwrap();
        let first_install = fs::read(&target).unwrap();
        assert!(first_install.starts_with(user_content));
        assert!(first_install
            .windows(start.len())
            .any(|window| window == start.as_bytes()));
        assert!(first_install
            .windows(end.len())
            .any(|window| window == end.as_bytes()));

        apply_mapping(
            &fixture.ops,
            mapping,
            asset_bytes(mapping.asset),
            &SystemTimestamp,
        )
        .unwrap();
        assert_eq!(fs::read(&target).unwrap(), first_install);

        let prefix = b"# before managed block\n";
        let suffix = b"# after managed block\n";
        let mut existing = prefix.to_vec();
        existing.extend_from_slice(start.as_bytes());
        existing.extend_from_slice(b"\nold managed contents\n");
        existing.extend_from_slice(end.as_bytes());
        existing.extend_from_slice(b"\n");
        existing.extend_from_slice(suffix);
        fs::write(&target, existing).unwrap();

        apply_mapping(
            &fixture.ops,
            mapping,
            asset_bytes(mapping.asset),
            &SystemTimestamp,
        )
        .unwrap();
        let mut expected = prefix.to_vec();
        expected.extend_from_slice(asset_bytes(mapping.asset));
        expected.extend_from_slice(suffix);
        assert_eq!(fs::read(&target).unwrap(), expected);
    }
}

#[test]
fn malformed_shell_managed_section_is_not_replaced() {
    let fixture = MappingFixture::with_platform(Platform::Linux);
    let mapping = shell_mapping("zsh", Platform::Linux);
    let target = fixture.ops.resolve_target(mapping.target).unwrap();
    let original = b"user content\n# >>> dotfiles managed zsh section >>>\nunfinished\n";
    fs::write(&target, original).unwrap();

    let error = apply_mapping(
        &fixture.ops,
        mapping,
        asset_bytes(mapping.asset),
        &SystemTimestamp,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("invalid managed-section boundary"));
    assert_eq!(fs::read(target).unwrap(), original);
}

fn shell_mapping(module_id: &str, platform: Platform) -> &'static MappingSpec {
    registry::find_module(module_id)
        .expect("shell module")
        .mappings
        .iter()
        .find(|mapping| mapping.platform == platform)
        .expect("shell profile mapping")
}

fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}
