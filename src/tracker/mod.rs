use super::common::SyncBackend;

pub use self::message::{Command, Request, Response};
pub use self::error::{TrackerError, TrackerErrorKind, TrackerResult};

pub mod message;
pub mod error;
pub mod threaded;

#[cfg(feature = "evented")] pub mod evented;

/// The tracker object.
pub struct Tracker;
// {
//     backend: SyncBackend,
// }

impl Tracker {
    pub fn new(_: SyncBackend) -> Tracker {
        Tracker
        // {
        //     backend: backend,
        // }
    }

    /// Handle a request.
    pub fn handle(&self, request: Request) -> Response {
        info!("request = {:?}", request);
        let response = self.dispatch_command(&request);
        info!("response = {:?}", response);
        response
    }

    fn dispatch_command(&self, request: &Request) -> Response {
        match request.op {
            _ => Response::from(
                TrackerError::unknown_command(
                    &format!("because f*** you, that's why. (command: {:?})", request.op))),
        }
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use super::*;
    use super::super::common::test_support::*;

    fn handler_fixture() -> Tracker {
        Tracker::new(sync_backend_fixture())
    }

    #[test]
    fn dispatch_unknown_command() {
        let handler = handler_fixture();
        let request = Request::from("this_command_doesnt_exist key1=val1&domain=foo".as_bytes());
        let result = handler.dispatch_command(&request);
        assert!(result.is_err());
        assert_eq!(TrackerErrorKind::UnknownCommand, result.unwrap_err().kind);
    }

    #[test]
    fn handle_unknown_command() {
        let response_re = Regex::new("^ERR unknown_command [^ ]+\r\n").unwrap();
        let handler = handler_fixture();
        let request = Request::from("this_command_doesnt_exist key1=val1&domain=foo".as_bytes());
        let response = handler.handle(request);
        let response_buf = response.render();
        let response_line = String::from_utf8_lossy(response_buf.as_ref());
        assert!(response_re.is_match(&response_line));
    }
}
