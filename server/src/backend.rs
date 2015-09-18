use mogilefs_common::MogResult;
use std::io::{Read, Write};
use time::Tm;
use url::Url;

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
