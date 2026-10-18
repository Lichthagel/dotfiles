#[allow(dead_code)]
pub mod installer;
#[allow(dead_code)]
pub mod mappings;
#[allow(dead_code)]
pub mod opencode;
#[allow(dead_code)]
pub mod runner;
#[allow(dead_code)]
pub mod secrets;

use std::ffi::OsString;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

#[allow(dead_code)]
static XDG_CONFIG_HOME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[allow(dead_code)]
pub struct ScopedXdgConfigHome {
    _lock: MutexGuard<'static, ()>,
    previous: Option<OsString>,
}

#[allow(dead_code)]
impl ScopedXdgConfigHome {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let lock = XDG_CONFIG_HOME_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", path.as_ref());
        Self {
            _lock: lock,
            previous,
        }
    }
}

#[allow(dead_code)]
impl Drop for ScopedXdgConfigHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}

#[allow(dead_code)]
pub mod assertions {
    pub fn assert_nonempty(bytes: &[u8], label: &str) {
        assert!(!bytes.is_empty(), "expected nonempty bytes for {label}");
    }

    pub fn assert_starts_with(bytes: &[u8], prefix: &[u8], label: &str) {
        assert!(bytes.starts_with(prefix), "unexpected prefix for {label}");
    }
}
