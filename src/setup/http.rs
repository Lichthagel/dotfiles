use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::error::{InstallError, Result};

pub const MAX_THEME_TREE_BYTES: usize = 1_048_576;
pub const MAX_THEME_FILE_BYTES: usize = 1_048_576;
const MAX_DIAGNOSTIC_BYTES: u64 = 8192;

#[cfg(test)]
mod tests;

pub trait HttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone)]
pub struct CurlHttpClient {
    program: PathBuf,
    timeout: Duration,
}

impl Default for CurlHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CurlHttpClient {
    pub fn new() -> Self {
        Self {
            program: if cfg!(windows) { "curl.exe" } else { "curl" }.into(),
            timeout: Duration::from_secs(60),
        }
    }

    fn command(&self, url: &str, limit: usize) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args([
                // This must be first: user curlrc files must not alter download policy.
                "--disable",
                "--silent",
                "--show-error",
                "--fail",
                "--location",
                "--max-redirs",
                "10",
                "--connect-timeout",
                "10",
                "--globoff",
                "--proto",
                "=http,https",
                "--proto-redir",
                "=http,https",
                "--user-agent",
                "dotfiles-rust-installer",
            ])
            .arg("--max-time")
            .arg(format!(
                "{}.{:03}",
                self.timeout.as_secs(),
                self.timeout.subsec_millis()
            ))
            .arg("--max-filesize")
            .arg(limit.to_string())
            .arg("--url")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn get_limited(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        let mut child = self.command(url, limit).spawn().map_err(|error| {
            InstallError::Setup(format!(
                "download {url}: unable to run {} (install curl and ensure it is on PATH): {error}",
                self.program.display()
            ))
        })?;
        let stdout = child.stdout.take().expect("piped curl stdout");
        let stderr = child.stderr.take().expect("piped curl stderr");
        // Drain stderr concurrently so a full pipe cannot block the body download.
        let diagnostics = thread::Builder::new()
            .spawn(move || read_diagnostics(stderr))
            .map_err(|error| {
                let _ = child.kill();
                let _ = child.wait();
                InstallError::Setup(format!("start curl diagnostics reader for {url}: {error}"))
            })?;
        let mut bytes = Vec::new();
        let read = stdout.take(limit as u64 + 1).read_to_end(&mut bytes);
        let oversized = bytes.len() > limit;
        if oversized || read.is_err() {
            let _ = child.kill();
        }
        let status = child.wait();
        let diagnostics = diagnostics.join().map_err(|_| {
            InstallError::Setup(format!("read curl diagnostics for {url}: worker panicked"))
        })?;
        if oversized {
            return Err(size_limit_error(url, limit));
        }
        read.map_err(|error| InstallError::Setup(format!("read response from {url}: {error}")))?;
        let status = status.map_err(|error| {
            InstallError::Setup(format!("wait for curl downloading {url}: {error}"))
        })?;
        // Curl can reject Content-Length before emitting any body bytes.
        if status.code() == Some(63) {
            return Err(size_limit_error(url, limit));
        }
        if !status.success() {
            return Err(InstallError::Setup(format!(
                "download {url}: curl failed ({status}): {}",
                String::from_utf8_lossy(&diagnostics).trim()
            )));
        }
        Ok(bytes)
    }
}

fn read_diagnostics(mut reader: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = reader
        .by_ref()
        .take(MAX_DIAGNOSTIC_BYTES)
        .read_to_end(&mut bytes);
    let _ = io::copy(&mut reader, &mut io::sink());
    bytes
}

fn size_limit_error(url: &str, limit: usize) -> InstallError {
    InstallError::Setup(format!("download {url} exceeds the {limit}-byte limit"))
}

impl HttpClient for CurlHttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let bytes = self.get_limited(url, MAX_THEME_TREE_BYTES)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| InstallError::Setup(format!("parse JSON from {url}: {error}")))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.get_limited(url, MAX_THEME_FILE_BYTES)
    }
}
