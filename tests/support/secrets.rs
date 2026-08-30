use std::path::{Path, PathBuf};

use age::secrecy::ExposeSecret;
use age::x25519;
use dotfiles::secrets::{IdentitySource, SecretValues};
use zeroize::Zeroizing;

pub struct EncryptedFixture {
    payload: Vec<u8>,
    identity: IdentitySource,
    identity_text: String,
    temp_dir: tempfile::TempDir,
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
            temp_dir: tempfile::tempdir().unwrap(),
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

    pub fn temp_plaintext_files(&self) -> Vec<PathBuf> {
        std::fs::read_dir(self.temp_dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.contains(".env") || name.contains("identity"))
            })
            .collect()
    }
}

pub fn synthetic_values(pairs: &[(&str, &str)]) -> SecretValues {
    SecretValues::from_pairs(pairs.iter().copied()).unwrap()
}
