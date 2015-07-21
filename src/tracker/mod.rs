use super::common::SyncBackend;
use super::error::{MogError, MogResult};
use super::storage::Storage;

pub use self::message::{Command, Request, Response};

pub mod message;
pub mod threaded;

#[cfg(feature = "evented")] pub mod evented;

/// The tracker object.
pub struct Tracker {
    backend: SyncBackend,
    storage: Storage,
}

impl Tracker {
    pub fn new(backend: SyncBackend, storage: Storage) -> Tracker {
        Tracker {
            backend: backend,
            storage: storage,
        }
    }

    /// Handle a request.
    pub fn handle(&self, request: Request) -> MogResult<Response> {
        info!("request = {:?}", request);
        let response = self.dispatch_command(&request);
        info!("response = {:?}", response);
        response
    }

    fn dispatch_command(&self, request: &Request) -> MogResult<Response> {
        use self::message::Command::*;

        match request.op {
            CreateDomain => self.create_domain(request),
            CreateOpen => self.create_open(request),
            CreateClose => self.create_close(request),
            Noop => self.noop(request),
            // _ => Err(MogError::UnknownCommand(Some(format!("{}", request.op)))),
        }
    }

    fn noop(&self, _request: &Request) -> MogResult<Response> {
        Ok(Response::new(vec![]))
    }

    fn create_domain(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::UnknownDomain(None)));
        try!(self.backend.create_domain(domain));
        Ok(Response::new(vec![ ("domain".to_string(), domain.to_string()) ]))
    }

    fn create_open(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::UnknownDomain(None)));
        let key = try!(args.get("key").ok_or(MogError::UnknownKey(None)));
        let urls = try!(self.backend.create_open(domain, key, &self.storage));
        let mut response_args = vec![];
        response_args.push(("dev_count".to_string(), urls.len().to_string()));
        for (i, url) in urls.iter().enumerate() {
            response_args.push((format!("devid_{}", i+1), (i+1).to_string()));
            response_args.push((format!("path_{}", i+1), url.to_string()));
        }
        Ok(Response::new(response_args))
    }

    fn create_close(&self, _request: &Request) -> MogResult<Response> {
        // There actually are implementations of this on the backend,
        // but they don't do anything at the moment, and there's not
        // much point in writing code here if it's not going to be
        // used. We'll just leave this blank for now.
        Ok(Response::new(vec![]))
    }
}
