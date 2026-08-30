use crate::error::Result;

pub trait HttpClient {
    fn get_json(&self, url: &str) -> Result<serde_json::Value>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;
}
