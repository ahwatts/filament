use chrono::UTC;
use mogilefs_common::{Backend, MogError, MogResult, Request, Response, FromBytes};
use statsd;
use std::sync::Mutex;

pub mod evented;
pub mod threaded;

/// The tracker object.
pub struct Tracker<B: Backend> {
    backend: B,
    statsd: Option<Mutex<statsd::Client>>,
}

impl<B: Backend> Tracker<B> {
    /// Create a new Tracker around a particular Backend.
    pub fn new(backend: B) -> Tracker<B> {
        Tracker {
            backend: backend,
            statsd: None,
        }
    }

    pub fn report_stats_to(&mut self, host: &str, prefix: &str) -> MogResult<()> {
        debug!("Reporting stats to statsd at {:?} with prefix {:?}", host, prefix);
        match statsd::Client::new(host, prefix) {
            Ok(s) => {
                self.statsd = Some(Mutex::new(s));
                Ok(())
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
        info!("request = {:?}", request);
        let start = UTC::now();

        self.with_statsd(|statsd| {
            let lock = UTC::now();
            let op_counter = format!("mogilefs_server.tracker.requests.{}", request.op());
            statsd.incr(&op_counter);

            let lock_time_counter = format!("mogilefs_server.tracker.statsd.lock_wait_time.pre.{}", request.op());
            statsd.timer(&lock_time_counter, (lock - start).num_milliseconds() as f64);
        });

        let begin = UTC::now();
        let response = self.backend.handle(request);
        let end = UTC::now();

        self.with_statsd(|statsd| {
            let lock = UTC::now();
            if let Err(ref e) = response {
                let err_counter = format!("mogilefs_server.tracker.errors.{}", e.error_kind());
                statsd.incr(&err_counter);
            }

            let time_counter = format!("mogilefs_server.tracker.requests.timing.{}", request.op());
            statsd.timer(&time_counter, (end - begin).num_milliseconds() as f64);

            let lock_time_counter = format!("mogilefs_server.tracker.statsd.lock_wait_time.post.{}", request.op());
            statsd.timer(&lock_time_counter, (lock - end).num_milliseconds() as f64);
        });

        info!("response = {:?}", response);
        response
    }

    fn with_statsd<F>(&self, callback: F)
        where F: Fn(&mut statsd::Client)
    {
        if let Some(ref mutex) = self.statsd {
            let mut lock = match mutex.lock() {
                Ok(l) => l,
                Err(p) => p.into_inner(),
            };

            callback(&mut lock);
        }
    }
}
