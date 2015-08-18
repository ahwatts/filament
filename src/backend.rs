use std::fmt::Debug;
use super::error::MogResult;
use time::Tm;
use url::Url;

pub trait Backend: Send + Sync + Debug {
    fn create_domain(&mut self, domain: &str) -> MogResult<()>;

    fn create_open(&mut self, domain: &str, key: &str) -> MogResult<Vec<Url>>;
    fn create_close(&mut self, domain: &str, key: &str, path: &Url, size: u64) -> MogResult<()>;
    fn get_paths(&self, domain: &str, key: &str) -> MogResult<Vec<Url>>;
    fn delete(&mut self, domain: &str, key: &str) -> MogResult<()>;
    fn rename(&mut self, domain: &str, from: &str, to: &str) -> MogResult<()>;

    fn list_keys(&self, domain: &str, prefix: Option<&str>, after_key: Option<&str>, limit: Option<usize>) -> MogResult<Vec<String>>;
}

#[derive(Debug)]
pub struct FileMetadata {
    pub size: usize,
    pub mtime: Tm,
}
