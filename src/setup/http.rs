use std::io::Read;
use std::time::Duration;

use crate::error::{InstallError, Result};

pub const MAX_THEME_TREE_BYTES: usize = 1_048_576;
pub const MAX_THEME_FILE_BYTES: usize = 1_048_576;

#[cfg(test)]
mod tests;

pub trait HttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone)]
pub struct UreqHttpClient {
    client: ureq::Agent,
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqHttpClient {
    pub fn new() -> Self {
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let client = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(60)))
            .max_redirects(10)
            .user_agent("dotfiles-rust-installer")
            .build()
            .into();
        Self { client }
    }

    fn get(&self, url: &str) -> Result<ureq::http::Response<ureq::Body>> {
        self.client
            .get(url)
            .call()
            .map_err(|error| InstallError::Setup(format!("download {url}: {error}")))
    }

    fn get_limited(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        let mut response = self.get(url)?;
        let content_length = response.body().content_length();
        if content_length.is_some_and(|length| length > limit as u64) {
            return Err(InstallError::Setup(format!(
                "download {url} exceeds the {}-byte limit",
                limit
            )));
        }
        let mut bytes = Vec::with_capacity(content_length.unwrap_or(0) as usize);
        response
            .body_mut()
            .as_reader()
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| InstallError::Setup(format!("read response from {url}: {error}")))?;
        if bytes.len() > limit {
            return Err(InstallError::Setup(format!(
                "download {url} exceeds the {}-byte limit",
                limit
            )));
        }
        Ok(bytes)
    }
}

impl HttpClient for UreqHttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let bytes = self.get_limited(url, MAX_THEME_TREE_BYTES)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| InstallError::Setup(format!("parse JSON from {url}: {error}")))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.get_limited(url, MAX_THEME_FILE_BYTES)
    }
}
