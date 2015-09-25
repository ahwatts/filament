use super::error::MogResult;
use super::request::{Request, Response};
use super::requests::*;

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

    fn handle<R: Request + ?Sized>(&self, request: &R) -> MogResult<Response> where Self: Sized {
        request.perform(self)
    }
}

/// Middleware that wraps the handling of a Request.
pub trait AroundMiddleware {
    fn around(self, backend: Box<Backend>) -> Box<Backend>;
}

/// A middleware stack wrapping a Backend.
pub struct BackendStack {
    backend: Option<Box<Backend>>,
}

impl BackendStack {
    pub fn new<B: Backend + 'static>(backend: B) -> BackendStack {
        BackendStack {
            backend: Some(Box::new(backend) as Box<Backend>),
        }
    }

    pub fn around<A: AroundMiddleware>(&mut self, around: A) {
        let mut backend = self.backend.take().unwrap();
        backend = around.around(backend);
        self.backend = Some(backend);
    }
}

impl Backend for BackendStack {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<CreateDomain> {
        self.backend.as_ref().unwrap().create_domain(req)
    }

    fn create_open(&self, req: &CreateOpen) -> MogResult<CreateOpenResponse> {
        self.backend.as_ref().unwrap().create_open(req)
    }

    fn create_close(&self, req: &CreateClose) -> MogResult<()> {
        self.backend.as_ref().unwrap().create_close(req)
    }

    fn get_paths(&self, req: &GetPaths) -> MogResult<GetPathsResponse> {
        self.backend.as_ref().unwrap().get_paths(req)
    }

    fn file_info(&self, req: &FileInfo) -> MogResult<FileInfoResponse> {
        self.backend.as_ref().unwrap().file_info(req)
    }

    fn delete(&self, req: &Delete) -> MogResult<()> {
        self.backend.as_ref().unwrap().delete(req)
    }

    fn rename(&self, req: &Rename) -> MogResult<()> {
        self.backend.as_ref().unwrap().rename(req)
    }

    fn list_keys(&self, req: &ListKeys) -> MogResult<ListKeysResponse> {
        self.backend.as_ref().unwrap().list_keys(req)
    }
}
