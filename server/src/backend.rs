use mogilefs_common::{MogResult, Request};
use mogilefs_common::requests::*;
use std::io::{Read, Write};
use time::Tm;
use url::Url;

pub trait TrackerBackend: Send + Sync {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<<CreateDomain as Request>::ResponseType>;

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

pub trait StorageBackend: Send + Sync {
    fn url_for_key(&self, domain: &str, key: &str) -> Url;

    fn file_metadata(&self, domain: &str, key: &str) -> MogResult<StorageMetadata>;
    fn store_reader_content<R: Read>(&self, domain: &str, key: &str, reader: &mut R) -> MogResult<()>;
    fn store_bytes_content(&self, domain: &str, key: &str, content: &[u8]) -> MogResult<()>;
    fn get_content<W: Write>(&self, domain: &str, key: &str, writer: &mut W) -> MogResult<()>;
}

#[derive(Debug)]
pub struct StorageMetadata {
    pub size: u64,
    pub mtime: Tm,
    // Etag, Content-Type?
}
