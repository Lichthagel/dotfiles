use std::time::Duration;

use crate::error::{InstallError, Result};

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
}

impl HttpClient for ReqwestHttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        self.get(url)?
            .json()
            .map_err(|error| InstallError::Setup(format!("parse JSON from {url}: {error}")))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.get(url)?
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| InstallError::Setup(format!("read response from {url}: {error}")))
    }
}
