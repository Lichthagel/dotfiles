use std::io::Read;
use std::time::Duration;

use crate::error::{InstallError, Result};

pub const MAX_THEME_TREE_BYTES: usize = 1_048_576;
pub const MAX_THEME_FILE_BYTES: usize = 1_048_576;

pub trait HttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    client: reqwest::blocking::Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent("dotfiles-rust-installer")
            .build()
            .map_err(|error| InstallError::Setup(format!("create HTTPS client: {error}")))?;
        Ok(Self { client })
    }

    fn get(&self, url: &str) -> Result<reqwest::blocking::Response> {
        self.client
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|error| InstallError::Setup(format!("download {url}: {error}")))
    }

    fn get_limited(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        let mut response = self.get(url)?;
        if response
            .content_length()
            .is_some_and(|length| length > limit as u64)
        {
            return Err(InstallError::Setup(format!(
                "download {url} exceeds the {}-byte limit",
                limit
            )));
        }
        let mut bytes = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
        response
            .by_ref()
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

impl HttpClient for ReqwestHttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let bytes = self.get_limited(url, MAX_THEME_TREE_BYTES)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| InstallError::Setup(format!("parse JSON from {url}: {error}")))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.get_limited(url, MAX_THEME_FILE_BYTES)
    }
}
