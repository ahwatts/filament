use mogilefs_common::MogResult;
use mogilefs_common::requests::*;
use std::io::{Read, Write};
use time::Tm;
use url::Url;

pub trait TrackerBackend: Send + Sync {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<CreateDomain>;
    fn create_open  (&self, req: &CreateOpen)   -> MogResult<CreateOpenResponse>;
    fn create_close (&self, req: &CreateClose)  -> MogResult<()>;
    fn get_paths    (&self, req: &GetPaths)     -> MogResult<GetPathsResponse>;
    fn file_info    (&self, req: &FileInfo)     -> MogResult<FileInfoResponse>;
    fn delete       (&self, req: &Delete)       -> MogResult<()>;
    fn rename       (&self, req: &Rename)       -> MogResult<()>;
    fn list_keys    (&self, req: &ListKeys)     -> MogResult<ListKeysResponse>;
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
