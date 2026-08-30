use std::fs;
use std::path::{Path, PathBuf};

use dotfiles::error::Result;
use dotfiles::model::{Platform, Target};
use dotfiles::platform::{
    replace_file_at, replace_file_at_with_timestamp, BackupRecord, PathContext, PlatformOps,
    TimestampProvider,
};
use dotfiles::registry::MappingSpec;
use tempfile::TempDir;

#[derive(Clone, Debug)]
pub struct TestPathContext {
    context: PathContext,
    platform: Platform,
    fail_rename: bool,
}

impl TestPathContext {
    pub fn new(root: PathBuf, platform: Platform) -> Self {
        let home = root.join("home");
        let profile = (platform == Platform::Windows)
            .then(|| home.join("Documents/PowerShell/Microsoft.PowerShell_profile.ps1"));
        let backup_root = root.join("state/dotfiles/backups");
        Self {
            context: PathContext::new(home, profile, backup_root),
            platform,
            fail_rename: false,
        }
    }

    pub fn with_failing_rename(mut self) -> Self {
        self.fail_rename = true;
        self
    }

    pub fn context(&self) -> &PathContext {
        &self.context
    }
}

impl PlatformOps for TestPathContext {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn path_context(&self) -> &PathContext {
        &self.context
    }

    fn replace_file(&self, target: &Path, bytes: &[u8]) -> Result<BackupRecord> {
        replace_file_at(&self.context, target, bytes, self.fail_rename)
    }

    fn replace_file_with_timestamp(
        &self,
        target: &Path,
        bytes: &[u8],
        timestamp: &dyn TimestampProvider,
    ) -> Result<BackupRecord> {
        replace_file_at_with_timestamp(&self.context, target, bytes, self.fail_rename, timestamp)
    }

    fn set_private_file_permissions(&self, _path: &Path) -> Result<()> {
        Ok(())
    }
}

pub struct MappingFixture {
    #[allow(dead_code)]
    pub temp: TempDir,
    pub ops: TestPathContext,
}

impl MappingFixture {
    pub fn new() -> Self {
        Self::with_platform(current_platform())
    }

    pub fn with_failing_rename() -> Self {
        let mut fixture = Self::new();
        fixture.ops = fixture.ops.clone().with_failing_rename();
        fixture
    }

    pub fn with_platform(platform: Platform) -> Self {
        let temp = tempfile::tempdir().expect("create mapping fixture");
        let ops = TestPathContext::new(temp.path().to_path_buf(), platform);
        fs::create_dir_all(ops.context().home.clone()).expect("create fixture home");
        Self { temp, ops }
    }

    pub fn target(&self, relative: &str) -> PathBuf {
        self.ops.context().home.join(relative)
    }

    pub fn write_target(&self, relative: &str, bytes: &[u8]) {
        let target = self.target(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("create target parent");
        }
        fs::write(target, bytes).expect("write target");
    }

    pub fn read_target(&self, relative: &str) -> Vec<u8> {
        fs::read(self.target(relative)).expect("read target")
    }

    pub fn read_backup(&self, relative: &str) -> Vec<u8> {
        let root = &self.ops.context().backup_root;
        let mut matches = Vec::new();
        collect_named_files(root, Path::new(relative), &mut matches);
        assert_eq!(matches.len(), 1, "expected one backup for {relative}");
        fs::read(&matches[0]).expect("read backup")
    }
}

fn collect_named_files(root: &Path, relative: &Path, matches: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_named_files(&path, relative, matches);
        } else if path.ends_with(relative) {
            matches.push(path);
        }
    }
}

pub fn git_mapping() -> &'static MappingSpec {
    let module = dotfiles::registry::find_module("git").expect("git registry module");
    let platform = current_platform();
    module
        .mappings
        .iter()
        .find(|mapping| mapping.platform == platform)
        .expect("git mapping for current platform")
}

pub fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

pub fn mapping_target(ops: &dyn PlatformOps, mapping: &MappingSpec) -> PathBuf {
    ops.resolve_target(mapping.target)
        .expect("resolve mapping target")
}

pub fn profile_target(ops: &dyn PlatformOps, target: Target) -> PathBuf {
    ops.resolve_target(target).expect("resolve profile target")
}
