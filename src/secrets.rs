use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use age::x25519;
use zeroize::Zeroizing;

use crate::assets::encrypted_bundle_bytes;
use crate::cli::TerminalIo;
use crate::error::{InstallError, Result};
use crate::model::SecretBundle;
use crate::platform;

#[derive(Clone)]
pub enum IdentitySource {
    Direct(Zeroizing<String>),
    File(PathBuf),
    Default,
}

impl fmt::Debug for IdentitySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct(_) => formatter.write_str("Direct(<redacted>)"),
            Self::File(path) => formatter.debug_tuple("File").field(path).finish(),
            Self::Default => formatter.write_str("Default"),
        }
    }
}

impl IdentitySource {
    pub fn from_environment() -> Result<Self> {
        if let Some(identity) = std::env::var_os("AGE_IDENTITY") {
            let identity = identity
                .into_string()
                .map_err(|_| InstallError::Secret("AGE_IDENTITY is not valid UTF-8".into()))?;
            if !identity.trim().is_empty() {
                return Ok(Self::Direct(Zeroizing::new(identity)));
            }
        }
        if let Some(path) = std::env::var_os("AGE_IDENTITIES") {
            if !path.is_empty() {
                return Ok(Self::File(PathBuf::from(path)));
            }
        }
        Ok(Self::Default)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretMode {
    Required,
    Optional,
}

pub trait IdentityPrompt {
    fn prompt_identity(&mut self) -> Result<Option<Zeroizing<String>>>;
}

pub struct TerminalIdentityPrompt<'a> {
    terminal: &'a mut dyn TerminalIo,
}

impl<'a> TerminalIdentityPrompt<'a> {
    pub fn new(terminal: &'a mut dyn TerminalIo) -> Self {
        Self { terminal }
    }
}

impl IdentityPrompt for TerminalIdentityPrompt<'_> {
    fn prompt_identity(&mut self) -> Result<Option<Zeroizing<String>>> {
        self.terminal
            .read_line("Age identity file path or key (default not found): ")
            .map(|value| value.map(Zeroizing::new))
    }
}

pub struct ResolvedIdentities {
    identities: Vec<x25519::Identity>,
}

impl ResolvedIdentities {
    fn new(identities: Vec<x25519::Identity>) -> Self {
        Self { identities }
    }

    fn as_identity_iter(&self) -> impl Iterator<Item = &dyn age::Identity> {
        self.identities
            .iter()
            .map(|identity| identity as &dyn age::Identity)
    }

    pub fn as_identity_count(&self) -> usize {
        self.identities.len()
    }
}

pub fn resolve_identity(
    source: IdentitySource,
    mode: SecretMode,
    mut prompt: Option<&mut dyn IdentityPrompt>,
) -> Result<Option<ResolvedIdentities>> {
    let (identity_text, source_description) = match source {
        IdentitySource::Direct(identity) => (Some(identity), "AGE_IDENTITY".to_owned()),
        IdentitySource::File(path) => match std::fs::read_to_string(&path) {
            Ok(contents) => (Some(Zeroizing::new(contents)), path.display().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (None, path.display().to_string())
            }
            Err(error) => {
                return Err(InstallError::Secret(format!(
                    "unable to read age identity file {}: {error}",
                    path.display()
                )))
            }
        },
        IdentitySource::Default => {
            let path = platform::default_identity_path()?;
            match std::fs::read_to_string(&path) {
                Ok(contents) => (Some(Zeroizing::new(contents)), path.display().to_string()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    (None, path.display().to_string())
                }
                Err(error) => {
                    return Err(InstallError::Secret(format!(
                        "unable to read age identity file {}: {error}",
                        path.display()
                    )))
                }
            }
        }
    };

    let identity_text = match identity_text {
        Some(text) => text,
        None => {
            let prompted = match prompt.as_mut() {
                Some(prompt) => prompt.prompt_identity()?,
                None => None,
            };
            let Some(value) = prompted.filter(|value| !value.trim().is_empty()) else {
                return match mode {
                    SecretMode::Optional => Ok(None),
                    SecretMode::Required => Err(InstallError::Secret(format!(
                        "age identity not found ({source_description})"
                    ))),
                };
            };
            let value = Zeroizing::new(value.trim().to_owned());
            let prompted_source = if value.starts_with("AGE-SECRET-KEY-") {
                IdentitySource::Direct(value)
            } else {
                IdentitySource::File(PathBuf::from(value.as_str()))
            };
            return resolve_identity(prompted_source, mode, None);
        }
    };

    let identities = parse_identities(&identity_text)?;
    if identities.is_empty() {
        return match mode {
            SecretMode::Optional => Ok(None),
            SecretMode::Required => Err(InstallError::Secret(
                "age identity did not contain an X25519 identity".into(),
            )),
        };
    }
    Ok(Some(ResolvedIdentities::new(identities)))
}

pub fn decrypt_bundle(
    bundle: SecretBundle,
    ciphertext: &[u8],
    source: &IdentitySource,
    mode: SecretMode,
) -> Result<Option<SecretScope>> {
    decrypt_bundle_with_prompt(bundle, ciphertext, source, mode, None)
}

pub fn decrypt_embedded(
    bundle: SecretBundle,
    mode: SecretMode,
    prompt: Option<&mut dyn IdentityPrompt>,
) -> Result<Option<SecretScope>> {
    let source = IdentitySource::from_environment()?;
    decrypt_bundle_with_prompt(
        bundle,
        encrypted_bundle_bytes(bundle),
        &source,
        mode,
        prompt,
    )
}

pub fn decrypt_bundle_with_prompt(
    bundle: SecretBundle,
    ciphertext: &[u8],
    source: &IdentitySource,
    mode: SecretMode,
    prompt: Option<&mut dyn IdentityPrompt>,
) -> Result<Option<SecretScope>> {
    let Some(identities) = resolve_identity(source.clone(), mode, prompt)? else {
        return Ok(None);
    };
    let decryptor = age::Decryptor::new_buffered(ciphertext)
        .map_err(|error| decryption_error(bundle, error))?;
    let mut reader = decryptor
        .decrypt(identities.as_identity_iter())
        .map_err(|error| decryption_error(bundle, error))?;
    let mut plaintext = Zeroizing::new(Vec::new());
    std::io::Read::read_to_end(&mut reader, plaintext.as_mut())
        .map_err(|error| decryption_error(bundle, error))?;
    let values = parse_secret_values(&plaintext)?;
    Ok(Some(SecretScope { values }))
}

fn parse_identities(contents: &str) -> Result<Vec<x25519::Identity>> {
    let mut identities = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let identity = x25519::Identity::from_str(line)
            .map_err(|error| InstallError::Secret(format!("invalid age identity: {error}")))?;
        identities.push(identity);
    }
    Ok(identities)
}

fn decryption_error(bundle: SecretBundle, error: impl fmt::Display) -> InstallError {
    InstallError::secret(format!("unable to decrypt {bundle:?} bundle: {error}"))
}

pub struct SecretValues {
    values: BTreeMap<String, Zeroizing<String>>,
}

impl SecretValues {
    pub fn from_pairs<I, K, V>(pairs: I) -> Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut values = BTreeMap::new();
        for (key, value) in pairs {
            let key = key.into();
            validate_name(&key)?;
            if values
                .insert(key.clone(), Zeroizing::new(value.into()))
                .is_some()
            {
                return Err(InstallError::Secret(format!(
                    "duplicate secret name: {key}"
                )));
            }
        }
        Ok(Self { values })
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(|value| value.as_str())
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }
}

impl fmt::Debug for SecretValues {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretValues")
            .field("names", &self.values.keys().collect::<Vec<_>>())
            .field("values", &"<redacted>")
            .finish()
    }
}

pub struct SecretScope {
    values: SecretValues,
}

impl SecretScope {
    pub fn values(&self) -> &SecretValues {
        &self.values
    }
}

impl fmt::Debug for SecretScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretScope")
            .field("values", &self.values)
            .finish()
    }
}

pub fn required_values(values: &SecretValues, names: &[&str]) -> Result<()> {
    let missing: Vec<_> = names
        .iter()
        .copied()
        .filter(|name| values.get(name).is_none_or(str::is_empty))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(InstallError::Secret(format!(
            "required secret values are missing or empty: {}",
            missing.join(", ")
        )))
    }
}

fn parse_secret_values(plaintext: &[u8]) -> Result<SecretValues> {
    let contents = std::str::from_utf8(plaintext).map_err(|error| {
        InstallError::Secret(format!("decrypted secrets are not UTF-8: {error}"))
    })?;
    let mut pairs = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(InstallError::Secret("invalid secrets format".into()));
        };
        let key = key.trim();
        validate_name(key)?;
        pairs.push((key.to_owned(), value.to_owned()));
    }
    SecretValues::from_pairs(pairs)
}

fn validate_name(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric());
    if valid {
        Ok(())
    } else {
        Err(InstallError::Secret(format!("invalid secret name: {name}")))
    }
}
