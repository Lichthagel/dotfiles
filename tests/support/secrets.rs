use std::path::{Path, PathBuf};

use age::secrecy::ExposeSecret;
use age::x25519;
use dotfiles::secrets::{IdentitySource, SecretValues};
use zeroize::Zeroizing;

pub struct EncryptedFixture {
    payload: Vec<u8>,
    identity: IdentitySource,
    identity_text: String,
}

pub struct TempRootGuard {
    root: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EncryptedFixture {
    pub fn new(plaintext: &str) -> Self {
        let identity = x25519::Identity::generate();
        let payload = age::encrypt(&identity.to_public(), plaintext.as_bytes()).unwrap();
        let identity_text = identity.to_string().expose_secret().to_owned();
        Self {
            payload,
            identity: IdentitySource::Direct(Zeroizing::new(identity_text.clone())),
            identity_text,
        }
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn identity(&self) -> IdentitySource {
        self.identity.clone()
    }

    pub fn identity_text(&self) -> &str {
        &self.identity_text
    }

    pub fn write_identity(&self, path: &Path) {
        std::fs::write(path, self.identity_text()).unwrap();
    }
}

impl TempRootGuard {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut previous = Vec::new();
        for name in ["TMPDIR", "TMP", "TEMP"] {
            let previous_value = std::env::var_os(name);
            std::env::set_var(name, root.path());
            previous.push((name, previous_value));
        }
        Self { root, previous }
    }

    pub fn temp_plaintext_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        collect_secret_artifacts(self.root.path(), &mut files);
        files
    }
}

impl Drop for TempRootGuard {
    fn drop(&mut self) {
        for (name, value) in self.previous.drain(..).rev() {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

fn collect_secret_artifacts(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_secret_artifacts(&path, files);
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.contains(".env") || name.to_ascii_lowercase().contains("identity") {
            files.push(path);
        }
    }
}

pub fn synthetic_values(pairs: &[(&str, &str)]) -> SecretValues {
    SecretValues::from_pairs(pairs.iter().copied()).unwrap()
}
