use super::error::MogResult;
use super::request::Response;
use super::requests::*;

/// Something which can manipulate a `Backend`, and produce a
/// `Response`.
pub trait Operation<On: ?Sized> {
    fn operate(&self, &On) -> MogResult<Response>;
}

/// A backend for the trackers.
pub trait Backend: Send + Sync {
    fn create_domain(&self, &CreateDomain) -> MogResult<CreateDomain>;
    fn create_open  (&self, &CreateOpen)   -> MogResult<CreateOpenResponse>;
    fn create_close (&self, &CreateClose)  -> MogResult<()>;
    fn get_paths    (&self, &GetPaths)     -> MogResult<GetPathsResponse>;
    fn file_info    (&self, &FileInfo)     -> MogResult<FileInfoResponse>;
    fn delete       (&self, &Delete)       -> MogResult<()>;
    fn rename       (&self, &Rename)       -> MogResult<()>;
    fn list_keys    (&self, &ListKeys)     -> MogResult<ListKeysResponse>;
}
