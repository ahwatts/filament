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
    pub fn handle_bytes(&self, request_bytes: &[u8]) -> MogResult<Response> {
        let request_result = Request::from_bytes(request_bytes);
        info!("request = {:?}", request_result);
        let response_result = request_result.and_then(|req| self.handle(&req));
        info!("response = {:?}", response_result);
        response_result
    }

    pub fn handle(&self, request: &Request) -> MogResult<Response> {
        use self::message::Command::*;

        match request.op {
            CreateDomain => self.create_domain(request),

            CreateOpen => self.create_open(request),
            CreateClose => self.create_close(request),
            GetPaths => self.get_paths(request),
            Delete => self.delete(request),
            ListKeys => self.list_keys(request),

            Noop => self.noop(request),
            // _ => Err(MogError::UnknownCommand(Some(format!("{}", request.op)))),
        }
    }

    fn noop(&self, _request: &Request) -> MogResult<Response> {
        Ok(Response::new(vec![]))
    }

    fn create_domain(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::NoDomain));
        try!(self.backend.create_domain(domain));
        Ok(Response::new(vec![ ("domain".to_string(), domain.to_string()) ]))
    }

    fn create_open(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::NoDomain));
        let key = try!(args.get("key").ok_or(MogError::NoKey));
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

    // request = "get_paths domain=rn_development_private&key=Song/512428/image&noverify=1&zone=\r\n"
    // response = "OK paths=1&path1=http://127.0.0.1:7500/dev1/0/000/000/0000000109.fid\r\n"
    fn get_paths(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::NoDomain));
        let key = try!(args.get("key").ok_or(MogError::NoKey));

        let paths = try!(self.backend.get_paths(domain, key, &self.storage));
        let mut response_args = vec![ ("paths".to_string(), paths.len().to_string()) ];
        for (i, url) in paths.iter().enumerate() {
            response_args.push((format!("path{}", i+1), url.to_string()));
        }
        Ok(Response::new(response_args))
    }

    // request = "rename domain=rn_development_private&from_key=Song/512428/image&to_key=Song/512428/image/1\r\n"
    // response = "OK \r\n"
    // request = "rename domain=rn_development_private&from_key=Song/9381/image&to_key=Song/512428/image/1\r\n"
    // response = "ERR key_exists Target+key+name+already+exists%3B+can%27t+overwrite.\r\n"
    // request = "rename domain=rn_development_private&from_key=Song/512428/image&to_key=Song/512428/image/1\r\n"
    // response = "ERR unknown_key unknown_key\r\n"

    fn delete(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::NoDomain));
        let key = try!(args.get("key").ok_or(MogError::NoKey));
        try!(self.backend.delete(domain, key));
        Ok(Response::new(vec![]))
    }

    fn list_keys(&self, request: &Request) -> MogResult<Response> {
        let args = request.args_hash();
        let domain = try!(args.get("domain").ok_or(MogError::NoDomain));
        let limit = args.get("limit").map(|lim| usize::from_str_radix(lim, 10).unwrap_or(1000));
        let after = args.get("after").map(|a| *a);
        let keys = try!(self.backend.list_keys(domain, None, after, limit));

        let mut response_args = vec![ ("key_count".to_string(), keys.len().to_string()) ];
        for (i, key) in keys.iter().enumerate() {
            response_args.push((format!("key_{}", i+1), key.to_string()));
            if i == keys.len() - 1 {
                response_args.push(("next_after".to_string(), key.to_string()));
            }
        }

        Ok(Response::new(response_args))
    }
}
