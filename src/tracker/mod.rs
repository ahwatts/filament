use super::common::SyncBackend;
use super::error::{MogError, MogResult};

pub use self::message::{Command, Request, Response};

pub mod message;
pub mod threaded;

#[cfg(feature = "evented")] pub mod evented;

/// The tracker object.
pub struct Tracker;

impl Tracker {
    pub fn new(_: SyncBackend) -> Tracker {
        Tracker
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
            // CreateOpen => self.create_open(request),
            _ => Err(MogError::UnknownCommand(Some(format!("{}", request.op)))),
        }
    }

    // fn create_open(&self, request: &Request) -> MogResult<Response> {
    // }
}
