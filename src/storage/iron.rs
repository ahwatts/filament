use iron::{Handler, IronResult, Request, Response};
use iron::method::Method;
use iron::status::Status;
use std::error::Error;
use super::Storage;

pub struct StorageHandler {
    store: Storage,
}

impl StorageHandler {
    pub fn new(storage: Storage) -> StorageHandler {
        StorageHandler {
            store: storage,
        }
    }

    fn handle_get(&self, key: &str) -> Result<Vec<u8>, String> {
        let mut content = vec![];
        match self.store.get_content(key, &mut content) {
            Ok(_) => Ok(content),
            Err(e) => Err(e.description().to_string()),
        }
    }
}

impl Handler for StorageHandler {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let key = request.url.path.connect("/");
        info!("Storage request: {:?} {} from {:?}", request.method, key, request.remote_addr);

        match request.method {
            Method::Get | Method::Head => {
                match self.handle_get(&key) {
                    Ok(data) => Ok(Response::with((Status::Ok, data))),
                    Err(s) => Ok(Response::with((Status::BadRequest, s))),
                }
            },
            // Method::Put => {},
            // Method::Delete => {},
            _ => Ok(Response::with((Status::BadRequest, "Unknown request type.\n"))),
        }
    }
}
