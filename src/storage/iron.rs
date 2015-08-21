use iron::{Handler, IronError, IronResult, Request, Response};
use iron::headers;
use iron::method::Method;
use iron::modifiers::Header;
use iron::status::Status;
use std::error::Error;
use std::ops::Deref;
use super::Storage;
use super::super::error::MogError;

pub struct StorageHandler {
    store: Storage,
}

impl StorageHandler {
    pub fn new(storage: Storage) -> StorageHandler {
        StorageHandler {
            store: storage,
        }
    }

    fn handle_get(&self, _request: &Request, domain: &str, key: &str) -> IronResult<Response> {
        let metadata = try!(self.store.file_metadata(domain, key));
        let mut content = vec![];
        try!(self.store.get_content(domain, key, &mut content));

        Ok(Response::with((
            Status::Ok,
            Header(headers::LastModified(headers::HttpDate(metadata.mtime))),
            Header(headers::ContentLength(metadata.size as u64)),
            content,)))
    }

    fn handle_put(&self, request: &mut Request, domain: &str, key: &str) -> IronResult<Response> {
        match self.store.store_content(domain, key, &mut request.body) {
            Ok(_) => Ok(Response::with((Status::Ok,))),
            Err(MogError::UnknownKey(ref k)) => {
                return Ok(Response::with((Status::NotFound, format!("Unknown key: {:?}\n", k))));
            },
            Err(e) => {
                let modifier = (Status::InternalServerError, format!("{}\n", e.description()));
                return Err(IronError::new(e, modifier));
            },
        }
    }
}

impl Handler for StorageHandler {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let dk = domain_and_key_from_path(&request.url.path);

        if dk.is_err() {
            info!("BAD Storage request: {:?} {:?} (body = {} bytes) from {:?}",
                  request.method, request.url,
                  request.headers.get::<headers::ContentLength>().map(|h| h.deref()).unwrap_or(&0),
                  request.remote_addr);
            return Ok(Response::with((Status::BadRequest, format!("{}\n", dk.unwrap_err()))));
        }

        let (domain, key) = dk.unwrap();
        info!("Storage request: {:?} domain = {} key = {} (body = {} bytes) from {:?}",
              request.method, domain, key,
              request.headers.get::<headers::ContentLength>().map(|h| h.deref()).unwrap_or(&0),
              request.remote_addr);

        let response_rslt = match request.method {
            Method::Get | Method::Head => self.handle_get(request, &domain, &key),
            Method::Put => self.handle_put(request, &domain, &key),
            _ => Ok(Response::with((Status::BadRequest, "Unknown request type.\n"))),
        };

        match response_rslt {
            Ok(ref response) =>
                info!("Storage response: {:?} (body = {} bytes to {:?})",
                      response.status,
                      response.headers.get::<headers::ContentLength>().map(|h| h.deref()).unwrap_or(&0),
                      request.remote_addr),
            Err(ref e) =>
                info!("Storage response: {:?} (error response) body = {} bytes to {:?}",
                      e.response.status,
                      e.response.headers.get::<headers::ContentLength>().map(|h| h.deref()).unwrap_or(&0),
                      request.remote_addr),
        }

        response_rslt
    }
}

fn domain_and_key_from_path(path: &Vec<String>) -> Result<(String, String), String> {
    let d_index = path.iter().position(|p| p == "d");
    let k_index = path.iter().position(|p| p == "k");

    match (d_index, k_index) {
        (Some(d), Some(k)) => {
            let domain = path[(d+1)..k].connect("/");
            let key = path[(k+1)..].connect("/");
            (Ok((domain, key)))
        },
        _ => {
            Err(format!("Could not extract domain or key from path: {:?}", path))
        }
    }
}
