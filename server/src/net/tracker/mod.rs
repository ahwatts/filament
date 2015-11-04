use mogilefs_common::{Backend, MogError, MogResult, Request, Response, FromBytes};
use rand::{self, Rng};
use statsd;
use std::cell::RefCell;
use std::collections::HashMap;

pub mod evented;
pub mod threaded;

thread_local!{
    static STATSD: RefCell<HashMap<u64, statsd::Client>> = RefCell::new(HashMap::new())
}

/// The tracker object.
pub struct Tracker<B: Backend> {
    backend: B,
    statsd_key: u64,
}

impl<B: Backend> Tracker<B> {
    /// Create a new Tracker around a particular Backend.
    pub fn new(backend: B) -> Tracker<B> {
        let mut rng = rand::thread_rng();
        Tracker {
            backend: backend,
            statsd_key: rng.gen(),
        }
    }

    pub fn report_stats_to(&mut self, host: &str, prefix: &str) -> MogResult<()> {
        debug!("Reporting stats to statsd at {:?} with prefix {:?}", host, prefix);
        match statsd::Client::new(host, prefix) {
            Ok(s) => {
                STATSD.with(|sock_cell| {
                    sock_cell.borrow_mut().insert(self.statsd_key, s);
                    Ok(())
                })
            },
            Err(e) => {
                Err(MogError::Other("Statsd error".to_string(), Some(format!("{:?}", e))))
            }
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
        STATSD.with(|sock_cell| {
            info!("request = {:?}", request);

            let response = if let Some(mut s) = sock_cell.borrow_mut().get_mut(&self.statsd_key) {
                s.incr(&format!("mogilefs_server.tracker.requests.{}", request.op()));

                let rslt = self.backend.handle(request);

                if let Err(ref e) = rslt {
                    s.incr(&format!("mogilefs_server.tracker.errors.{}", e.error_kind()));
                }

                rslt
            } else {
                self.backend.handle(request)
            };

            info!("response = {:?}", response);
            response
        })
    }
}

impl<B: Backend> Drop for Tracker<B> {
    fn drop(&mut self) {
        STATSD.with(|sock_cell| {
            sock_cell.borrow_mut().remove(&self.statsd_key);
        })
    }
}
