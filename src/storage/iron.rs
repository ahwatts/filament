use iron::{Handler, IronError, IronResult, Request, Response};
use iron::headers;
use iron::method::Method;
use iron::modifier::Set;
use iron::modifiers::Header;
use iron::status::Status;
use std::error::Error;
use super::{Storage, StorageError, StorageResult};

pub struct StorageHandler {
    store: Storage,
}

impl StorageHandler {
    pub fn new(storage: Storage) -> StorageHandler {
        StorageHandler {
            store: storage,
        }
    }

    fn handle_get(&self, request: &Request, key: &str) -> IronResult<Response> {
        let mut content = vec![];
        match self.store.get_content(key, &mut content) {
            Ok(_) => {},
            Err(StorageError::UnknownKey) => {
                return Ok(Response::with((Status::NotFound, "Unknown key.\n")));
            },
            Err(StorageError::NoContent) => {
                return Ok(Response::with((Status::NotFound, "No content for key.\n")));
            },
            Err(e) => {
                let modifier = (Status::InternalServerError, format!("{}\n", e.description()));
                return Err(IronError::new(e, modifier));
            },
        };

        let mut response = Response::with((Status::Ok,));
        response = response.set(Header(headers::ContentLength(content.len() as u64)));

        if request.method == Method::Get {
            response = response.set(content);
        }

        Ok(response)
    }

    fn handle_put(&self, request: &mut Request, key: &str) -> IronResult<Response> {
        match self.store.store_content(key, &mut request.body) {
            Ok(_) => Ok(Response::with((Status::Ok,))),
            Err(StorageError::UnknownKey) => {
                return Ok(Response::with((Status::NotFound, "Unknown key.\n")));
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
        let key = request.url.path.connect("/");
        info!("Storage request: {:?} {} from {:?}", request.method, key, request.remote_addr);

        match request.method {
            Method::Get | Method::Head => self.handle_get(request, &key),
            Method::Put => self.handle_put(request, &key),
            _ => Ok(Response::with((Status::BadRequest, "Unknown request type.\n"))),
        }
    }
}
