use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn module_legacy_files(root: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).expect("read module directory");
    for entry in entries {
        let path = entry.expect("read module entry").path();
        if path.is_dir() {
            module_legacy_files(&path, found);
            continue;
        }
        let legacy = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "module.conf" || name == "setup.sh" || name == "setup.ps1");
        if legacy {
            found.push(path);
        }
    }
}

#[test]
fn legacy_installer_surface_is_removed_and_embedded_secret_sources_remain() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "install.sh",
        "install.ps1",
        "lib/secrets.sh",
        "lib/secrets.ps1",
        "modules/manifest.conf",
        "modules/dependencies.conf",
    ] {
        assert!(
            !root.join(relative).exists(),
            "legacy path remains: {relative}"
        );
    }

    let mut legacy_modules = Vec::new();
    module_legacy_files(&root.join("modules"), &mut legacy_modules);
    assert!(
        legacy_modules.is_empty(),
        "legacy module files remain: {legacy_modules:?}"
    );

    for relative in ["secrets/atuin.env.age", "secrets/opencode.env.age"] {
        assert!(
            root.join(relative).is_file(),
            "embedded secret source missing: {relative}"
        );
    }
}

#[test]
fn no_plaintext_secret_environment_file_is_tracked() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("git")
        .args(["ls-files", "secrets/*.env"])
        .current_dir(root)
        .output()
        .expect("git must be available for repository secret checks");
    assert!(output.status.success(), "git ls-files failed: {output:?}");
    assert!(
        output.stdout.is_empty(),
        "tracked plaintext secret source: {:?}",
        output.stdout
    );
}
