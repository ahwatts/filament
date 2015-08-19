use std::fmt::Debug;
use super::error::MogResult;
use time::Tm;
use url::Url;

pub trait TrackerBackend: Send + Sync + Debug {
    fn create_domain(&self, domain: &str) -> MogResult<()>;

    fn create_open(&self, domain: &str, key: &str) -> MogResult<Vec<Url>>;
    fn create_close(&self, domain: &str, key: &str, path: &Url, size: u64) -> MogResult<()>;
    fn get_paths(&self, domain: &str, key: &str) -> MogResult<Vec<Url>>;
    fn file_info(&self, domain: &str, key: &str) -> MogResult<TrackerMetadata>;
    fn delete(&self, domain: &str, key: &str) -> MogResult<()>;
    fn rename(&self, domain: &str, from: &str, to: &str) -> MogResult<()>;

    fn list_keys(&self, domain: &str, prefix: Option<&str>, after_key: Option<&str>, limit: Option<usize>) -> MogResult<Vec<String>>;
}

#[derive(Debug)]
pub struct TrackerMetadata {
    pub size: u64,
    pub domain: String,
    pub key: String,
    // Also: class, devcount, fid?
}

pub trait StorageBackend: Send + Sync + Debug {}

#[derive(Debug)]
pub struct StorageMetadata {
    pub size: u64,
    pub mtime: Tm,
    // Etag, Content-Type?
}
