use mogilefs_common::{Backend, MogResult, Request, Response, FromBytes};

pub mod evented;
pub mod threaded;

/// The tracker object.
pub struct Tracker<B: Backend> {
    backend: B,
}

impl<B: Backend> Tracker<B> {
    pub fn new(backend: B) -> Tracker<B> {
        Tracker {
            backend: backend,
        }
    }

    /// Handle a request.
    pub fn handle_bytes(&self, request_bytes: &[u8]) -> MogResult<Response> {
        match Box::<Request>::from_bytes(request_bytes) {
            Ok(request) => self.handle_request(&*request),
            Err(e) => {
                error!("Error parsing request: {}, raw request = {:?}",
                       e, String::from_utf8_lossy(request_bytes));
                Err(e)
            }
        }
    }

    pub fn handle_request(&self, request: &Request) -> MogResult<Response> {
        info!("request = {:?}", request);
        let response = self.backend.perform(request);
        info!("response = {:?}", response);
        response
    }
}
