use iron::{Chain, Handler, Iron, IronResult, Request, Response};
use iron::status::Status;
use std::net::{SocketAddr, ToSocketAddrs};
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
}

impl Handler for StorageHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        Ok(Response::with((Status::Ok, "Hello, World!")))
    }
}
