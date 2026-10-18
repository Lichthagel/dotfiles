use std::path::PathBuf;

use dotfiles::cli::{TerminalIo, TerminalKey};
use dotfiles::engine::InstallContext;
use dotfiles::error::Result;
use dotfiles::model::Platform;
use dotfiles::platform::{PathContext, PlatformOps, TimestampProvider};
use dotfiles::setup::HttpClient;
use tempfile::TempDir;

use super::mappings::TestPathContext;
use super::runner::FakeRunner;

pub struct TestHttp;

impl HttpClient for TestHttp {
    fn get_json(&self, _url: &str) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "tree": [{"path": "themes/test.json", "type": "blob"}]
        }))
    }

    fn get_bytes(&self, _url: &str) -> Result<Vec<u8>> {
        Ok(br#"{"name":"test"}"#.to_vec())
    }
}

#[derive(Default)]
pub struct TestTerminal {
    lines: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    secret_reads: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl TestTerminal {
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    pub fn secret_reads(&self) -> usize {
        *self.secret_reads.lock().unwrap()
    }
}

impl TerminalIo for TestTerminal {
    fn is_interactive(&self) -> bool {
        false
    }

    fn read_key(&mut self) -> Result<TerminalKey> {
        unreachable!("noninteractive installer tests do not read keys")
    }

    fn read_secret(&mut self, _prompt: &str) -> Result<Option<zeroize::Zeroizing<String>>> {
        *self.secret_reads.lock().unwrap() += 1;
        Ok(None)
    }

    fn clear(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_line(&mut self, line: &str) -> Result<()> {
        self.lines.lock().unwrap().push(line.to_owned());
        Ok(())
    }
}

pub struct FixedTimestamp;

impl TimestampProvider for FixedTimestamp {
    fn timestamp(&self) -> String {
        "fixed-timestamp".into()
    }
}

pub struct InstallFixture {
    pub temp: TempDir,
    pub ops: TestPathContext,
    pub http: TestHttp,
    pub terminal: TestTerminal,
    pub timestamp: FixedTimestamp,
}

impl InstallFixture {
    pub fn new(platform: Platform) -> Self {
        let temp = tempfile::tempdir().expect("create installer fixture");
        let ops = TestPathContext::new(temp.path().to_path_buf(), platform);
        std::fs::create_dir_all(ops.path_context().home.clone()).expect("create fixture home");
        Self {
            temp,
            ops,
            http: TestHttp,
            terminal: TestTerminal::default(),
            timestamp: FixedTimestamp,
        }
    }

    pub fn context<'a>(&'a mut self, runner: &'a FakeRunner) -> InstallContext<'a> {
        InstallContext {
            platform: &self.ops,
            runner,
            http: &self.http,
            terminal: &mut self.terminal,
            timestamp: &self.timestamp,
        }
    }

    pub fn target(&self, relative: &str) -> PathBuf {
        self.ops.path_context().home.join(relative)
    }
}

pub fn test_context<'a>(
    fixture: &'a mut InstallFixture,
    runner: &'a FakeRunner,
) -> InstallContext<'a> {
    fixture.context(runner)
}

#[allow(dead_code)]
pub fn path_context(ops: &dyn PlatformOps) -> &PathContext {
    ops.path_context()
}
