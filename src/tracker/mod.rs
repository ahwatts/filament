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
        match request.op {
            _ => Err(MogError::UnknownCommand(Some(format!("{}", request.op)))),
        }
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use super::*;
    use super::super::common::test_support::*;
    use super::super::error::MogError;

    fn handler_fixture() -> Tracker {
        Tracker::new(sync_backend_fixture())
    }

    // #[test]
    // fn dispatch_unknown_command() {
    //     let handler = handler_fixture();
    //     let request = Request::from_bytes("this_command_doesnt_exist key1=val1&domain=foo".as_bytes());
    //     let result = handler.dispatch_command(&request);
    //     println!("result = {:?}", result);
    //     assert!(matches!(result, Err(MogError::UnknownCommand(Some(ref k))) if k == "this_command_doesnt_exist"));
    // }

    // #[test]
    // fn handle_unknown_command() {
    //     let response_re = Regex::new("^ERR unknown_command [^ ]+\r\n").unwrap();
    //     let handler = handler_fixture();
    //     let request = Request::from("this_command_doesnt_exist key1=val1&domain=foo".as_bytes());
    //     let response = Response::from(handler.handle(request));
    //     let response_buf = response.render();
    //     let response_line = String::from_utf8_lossy(response_buf.as_ref());
    //     assert!(response_re.is_match(&response_line));
    // }
}
