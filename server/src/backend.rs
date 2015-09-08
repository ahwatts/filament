use mogilefs_common::{MogResult, Request};
use mogilefs_common::requests::*;
use std::io::{Read, Write};
use time::Tm;
use url::Url;

pub trait TrackerBackend: Send + Sync {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<<CreateDomain as Request>::ResponseType>;
    fn create_open  (&self, req: &CreateOpen)   -> MogResult<<CreateOpen   as Request>::ResponseType>;
    fn create_close (&self, req: &CreateClose)  -> MogResult<<CreateClose  as Request>::ResponseType>;
    fn get_paths    (&self, req: &GetPaths)     -> MogResult<<GetPaths     as Request>::ResponseType>;
    fn file_info    (&self, req: &FileInfo)     -> MogResult<<FileInfo     as Request>::ResponseType>;
    fn delete       (&self, req: &Delete)       -> MogResult<<Delete       as Request>::ResponseType>;
    fn rename       (&self, req: &Rename)       -> MogResult<<Rename       as Request>::ResponseType>;
    fn list_keys    (&self, req: &ListKeys)     -> MogResult<<ListKeys     as Request>::ResponseType>;
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
