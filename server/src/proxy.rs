//! A backend implementation which proxies the requests to a set of
//! real trackers, possibly doing some monkeying with them in the
//! process.

use mogilefs_client::MogClient;
use mogilefs_common::requests::*;
use mogilefs_common::{Backend, Request, MogError, MogResult};
use std::any::Any;
use std::cell::RefCell;
use std::net::SocketAddr;

thread_local!{
    static CONNECTION: RefCell<Option<MogClient>> = RefCell::new(None)
}

/// The main `Backend` implementation for a proxy backend.
pub struct ProxyTrackerBackend {
    trackers: Vec<SocketAddr>,
}

impl ProxyTrackerBackend {
    pub fn new(trackers: &[SocketAddr]) -> MogResult<ProxyTrackerBackend> {
        let backend = ProxyTrackerBackend {
            trackers: trackers.to_owned(),
        };
        Ok(backend)
    }

    fn send_request<Req: Request + ?Sized, Res: Any>(&self, req: &Req) -> MogResult<Res> {
        CONNECTION.with(|conn_cell| {
            let mut conn_opt = conn_cell.borrow_mut();

            if conn_opt.is_none() {
                let client = MogClient::new(&self.trackers);
                *conn_opt = Some(client);
            }

            let conn = conn_opt.as_mut().unwrap();
            debug!("Sending request {:?} to {:?}", req, conn.peer_addr());
            let response_rslt = conn.request(req);
            debug!("Got response {:?} from {:?}", response_rslt, conn.peer_addr());
            response_rslt.and_then(|response| {
                response.downcast::<Res>().ok_or(MogError::BadResponse)
            })
        })
    }
}

impl Backend for ProxyTrackerBackend {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<CreateDomain> {
        self.send_request(req)
    }

    fn create_open(&self, req: &CreateOpen) -> MogResult<CreateOpenResponse> {
        self.send_request(req)
    }

    fn create_close(&self, req: &CreateClose) -> MogResult<()> {
        self.send_request(req)
    }

    fn create_class(&self, req: &CreateClass) -> MogResult<CreateClassResponse> {
        self.send_request(req)
    }

    fn get_paths(&self, req: &GetPaths) -> MogResult<GetPathsResponse> {
        self.send_request(req)
    }
    
    fn file_info(&self, req: &FileInfo) -> MogResult<FileInfoResponse> {
        self.send_request(req)
    }
    
    fn delete(&self, req: &Delete) -> MogResult<()> {
        self.send_request(req)
    }

    fn rename(&self, req: &Rename) -> MogResult<()> {
        self.send_request(req)
    }

    fn list_keys(&self, req: &ListKeys) -> MogResult<ListKeysResponse> {
        self.send_request(req)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{SocketAddr, ToSocketAddrs};
    use super::*;

    fn tracker_addr_list() -> Vec<SocketAddr> {
        let rv: Vec<SocketAddr> = vec![ "127.0.0.1:7101", "127.0.0.1:7102", "[::1]:7103" ]
            .iter()
            .flat_map(|a| a.to_socket_addrs().unwrap())
            .collect();
        assert_eq!(3, rv.len());
        rv
    }

    #[test]
    fn initial_state() {
        let addr_list = tracker_addr_list();
        let backend = ProxyTrackerBackend::new(&addr_list).unwrap();
        assert_eq!(addr_list, backend.trackers);
    }
}
