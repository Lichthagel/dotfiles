use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::error::{InstallError, Result};
use crate::platform::PlatformOps;

#[derive(Debug, Deserialize, Serialize)]
pub struct ModuleState {
    schema_version: u32,
    modules: BTreeSet<String>,
}

impl Default for ModuleState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            modules: BTreeSet::new(),
        }
    }
}

impl ModuleState {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(state_error(path, error)),
        };
        let state: Self = serde_json::from_slice(&bytes)
            .map_err(|_| state_error(path, "invalid module state JSON"))?;
        if state.schema_version != 1 {
            return Err(state_error(path, "unsupported module state schema version"));
        }
        Ok(state)
    }

    pub fn contains(&self, module: &str) -> bool {
        self.modules.contains(module)
    }
}

pub(crate) struct StateStore {
    path: PathBuf,
    state: ModuleState,
    _lock: File,
}

impl StateStore {
    pub(crate) fn open(platform: &dyn PlatformOps) -> Result<Self> {
        let path = platform.state_path()?;
        let parent = path
            .parent()
            .ok_or_else(|| state_error(&path, "no parent directory"))?;
        fs::create_dir_all(parent).map_err(|error| state_error(&path, error))?;
        let lock_path = path.with_extension("lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|error| state_error(&lock_path, error))?;
        // Keep the OS lock for the installation, including package and setup phases.
        lock.try_lock().map_err(|error| {
            state_error(
                &lock_path,
                format!("could not lock installation state: {error}"),
            )
        })?;
        let state = ModuleState::load(&path)?;
        Ok(Self {
            path,
            state,
            _lock: lock,
        })
    }

    pub(crate) fn contains(&self, module: &str) -> bool {
        self.state.contains(module)
    }

    pub(crate) fn record(&mut self, module: &str) -> Result<()> {
        if !self.state.modules.insert(module.to_owned()) {
            return Ok(());
        }
        let result = self.persist();
        if result.is_err() {
            self.state.modules.remove(module);
        }
        result
    }

    fn persist(&self) -> Result<()> {
        let parent = self.path.parent().expect("state path has a parent");
        let write = || -> Result<()> {
            let mut stage = NamedTempFile::new_in(parent)?;
            serde_json::to_writer_pretty(stage.as_file_mut(), &self.state)
                .map_err(|_| InstallError::State("could not serialize module state".into()))?;
            stage.write_all(b"\n")?;
            stage.as_file().sync_all()?;
            // Persist replaces the destination atomically on Linux and Windows.
            stage.persist(&self.path).map_err(|error| error.error)?;
            Ok(())
        };
        write().map_err(|error| state_error(&self.path, error))
    }
}

fn state_error(path: &Path, error: impl std::fmt::Display) -> InstallError {
    InstallError::State(format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Platform;
    use crate::platform::PathContext;

    struct TestPlatform(PathContext);

    impl PlatformOps for TestPlatform {
        fn platform(&self) -> Platform {
            Platform::Linux
        }

        fn path_context(&self) -> &PathContext {
            &self.0
        }

        fn set_private_file_permissions(&self, _path: &Path) -> Result<()> {
            Ok(())
        }
    }

    fn platform(root: &Path) -> TestPlatform {
        TestPlatform(PathContext::new(
            root.join("home"),
            None,
            root.join("state/dotfiles/backups"),
        ))
    }

    #[test]
    fn missing_state_is_empty_without_creating_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = platform(temp.path()).state_path().unwrap();
        assert!(!ModuleState::load(&path).unwrap().contains("git"));
        assert!(!path.parent().unwrap().exists());
    }

    #[test]
    fn records_are_sorted_unique_and_preserved_across_runs() {
        let temp = tempfile::tempdir().unwrap();
        let platform = platform(temp.path());
        {
            let mut store = StateStore::open(&platform).unwrap();
            store.record("mise").unwrap();
            store.record("git").unwrap();
            store.record("git").unwrap();
        }
        let mut store = StateStore::open(&platform).unwrap();
        store.record("bash").unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(platform.state_path().unwrap()).unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "schema_version": 1,
                "modules": ["bash", "git", "mise"]
            })
        );
    }

    #[test]
    fn invalid_or_unsupported_state_is_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let platform = platform(temp.path());
        let path = platform.state_path().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for bytes in [
            "not-json",
            r#"{"schema_version":2,"modules":["git"]}"#,
            r#"{"schema_version":1}"#,
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(StateStore::open(&platform).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), bytes);
        }
    }

    #[test]
    fn installation_lock_is_exclusive_and_released_on_drop() {
        let temp = tempfile::tempdir().unwrap();
        let platform = platform(temp.path());
        let store = StateStore::open(&platform).unwrap();
        assert!(StateStore::open(&platform).is_err());
        drop(store);
        assert!(StateStore::open(&platform).is_ok());
    }

    #[test]
    fn failed_replacement_preserves_existing_target_and_cleans_staging_file() {
        let temp = tempfile::tempdir().unwrap();
        let platform = platform(temp.path());
        let mut store = StateStore::open(&platform).unwrap();
        fs::create_dir(&store.path).unwrap();
        fs::write(store.path.join("keep"), b"original").unwrap();
        assert!(store.record("git").is_err());
        assert!(!store.state.contains("git"));
        assert_eq!(fs::read(store.path.join("keep")).unwrap(), b"original");
        assert_eq!(
            fs::read_dir(store.path.parent().unwrap()).unwrap().count(),
            2
        );
    }
}
