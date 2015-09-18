use super::error::MogResult;
use super::request::Response;
use super::requests::*;

pub trait Operation<On: ?Sized> {
    fn operate(&self, &On) -> MogResult<Response>;
}

pub trait Backend: Send + Sync {
    fn create_domain(&self, &CreateDomain) -> MogResult<CreateDomain>;
    fn create_open  (&self, &CreateOpen)   -> MogResult<CreateOpenResponse>;
    fn create_close (&self, &CreateClose)  -> MogResult<()>;
    fn get_paths    (&self, &GetPaths)     -> MogResult<GetPathsResponse>;
    fn file_info    (&self, &FileInfo)     -> MogResult<FileInfoResponse>;
    fn delete       (&self, &Delete)       -> MogResult<()>;
    fn rename       (&self, &Rename)       -> MogResult<()>;
    fn list_keys    (&self, &ListKeys)     -> MogResult<ListKeysResponse>;

    fn perform<Op: Operation<Self> + ?Sized>(&self, request: &Op) -> MogResult<Response> where Self: Sized {
        request.operate(self)
    }
}
