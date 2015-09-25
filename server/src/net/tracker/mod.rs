use mogilefs_common::{Backend, MogResult, Request, Response, FromBytes};

pub mod evented;
pub mod threaded;

/// The tracker object.
pub struct Tracker<B: Backend> {
    backend: B,
}

impl<B: Backend> Tracker<B> {
    /// Create a new Tracker around a particular Backend.
    pub fn new(backend: B) -> Tracker<B> {
        Tracker {
            backend: backend,
        }
    }

    /// Parse the bytes of a MogileFS request from the network into a
    /// Request, and hand that off to the Backend for processing.
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

    /// Handle a Request.
    pub fn handle_request(&self, request: &Request) -> MogResult<Response> {
        info!("request = {:?}", request);
        let response = self.backend.handle(request);
        info!("response = {:?}", response);
        response
    }
}
