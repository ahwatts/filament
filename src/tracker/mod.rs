pub use self::message::{Message, MessageBody, ToMessage};
pub use self::error::{TrackerError, TrackerErrorKind, TrackerResult};

pub mod message;
pub mod error;

#[cfg(not(windows))]
pub mod evented;

#[cfg(windows)]
pub mod threaded;

/// The tracker object.
pub struct Tracker;

impl Tracker {
    pub fn new() -> Tracker {
        Tracker
    }

    /// Handle a request.
    pub fn handle<R: ToMessage>(&self, request_in: R) -> Message {
        let request = match request_in.to_message() {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error reading MogileFS request: {}", e);
                return Message::from(e);
            }
        };

        info!("request = {:?}", request);
        let response = self.dispatch_command(&request);
        info!("response = {:?}", response);

        match response {
            Ok(msg) => msg,
            Err(e) => Message::from(e),
        }
    }

    fn dispatch_command(&self, request: &Message) -> TrackerResult<Message> {
        match request.op {
            _ => Err(TrackerError::unknown_command(format!("because f*** you, that's why. (command: {:?})", request.op).as_ref())),
        }
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use super::*;

    fn handler_fixture() -> Tracker {
        Tracker::new()
    }

    #[test]
    fn dispatch_unknown_command() {
        let handler = handler_fixture();
        let request = Message::from("this_command_doesnt_exist key1=val1&domain=foo".as_bytes());
        let result = handler.dispatch_command(&request);
        assert!(result.is_err());
        assert_eq!(TrackerErrorKind::UnknownCommand, result.unwrap_err().kind);
    }

    #[test]
    fn handle_unknown_command() {
        let response_re = Regex::new("^ERR unknown_command [^ ]+\r\n").unwrap();
        let handler = handler_fixture();
        let request = Message::from("this_command_doesnt_exist key1=val1&domain=foo".as_bytes());
        let response = handler.handle(request);
        let response_line = response.render();
        assert!(response_re.is_match(&response_line));
    }
}
