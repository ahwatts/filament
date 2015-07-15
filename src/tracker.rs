use std::collections::HashMap;
use std::convert::AsRef;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::result;
use url::{form_urlencoded, percent_encoding};

/// A request to or resposne from a MogileFS tracker.
#[derive(Debug)]
pub struct Message {
    pub op: String,
    pub body: MessageBody,
}

impl Message {
    pub fn args_hash<'a>(&'a self) -> HashMap<&'a str, &'a str> {
        match self.body {
            MessageBody::Args(ref args) => {
                args.iter().fold(HashMap::new(), |mut m, &(ref k, ref v)| {
                    *m.entry(k).or_insert(v) = v; m
                })
            },
            MessageBody::Message(_) => {
                HashMap::new()
            }
        }
    }

    pub fn render(&self) -> String {
        match self.body {
            MessageBody::Args(_) => format!("OK {} {}\r\n", self.op, self.body.render()),
            MessageBody::Message(_) => format!("ERR {} {}\r\n", self.op, self.body.render()),
        }
    }
}

impl<'a> From<&'a [u8]> for Message {
    fn from(bytes: &[u8]) -> Message {
        let mut toks = bytes.split(|&c| c == b' ');
        let command = String::from_utf8_lossy(toks.next().unwrap_or(b""));
        let parsed_args = form_urlencoded::parse(toks.next().unwrap_or(b""));

        let body = if parsed_args.len() == 1 && parsed_args[0].1 == "" {
            MessageBody::Message(parsed_args[0].0.clone())
        } else {
            MessageBody::Args(parsed_args)
        };

        Message {
            op: command.into_owned(),
            body: body,
        }
    }
}

impl From<Error> for Message {
    fn from(err: Error) -> Message {
        Message {
            op: format!("{}", err.kind),
            body: MessageBody::Message(err.description),
        }
    }
}

/// The body of a MogileFS request / response. It can either be a
/// query string or a url-encoded message.
#[derive(Debug)]
pub enum MessageBody {
    Args(Vec<(String, String)>),
    Message(String),
}

impl MessageBody {
    fn render(&self) -> String {
        match *self {
            MessageBody::Args(ref args) => form_urlencoded::serialize(args),
            MessageBody::Message(ref msg) => percent_encoding::percent_encode(
                msg.as_bytes(), percent_encoding::FORM_URLENCODED_ENCODE_SET),
        }
    }
}

/// Something that can be turned in to a MogileFS request or response.
pub trait ToMessage {
    fn to_message<'a>(self) -> Result<'a, Message>;
}

impl<R: Read> ToMessage for R {
    fn to_message<'a>(self) -> Result<'a, Message> {
        let mut reader = BufReader::new(self);
        let mut line: Vec<u8> = vec![];
        try!(reader.read_until(b'\r', &mut line));
        Ok(Message::from(line.as_ref()))
    }
}

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

    fn dispatch_command(&self, request: &Message) -> Result<Message> {
        match request.op {
            _ => Err(Error::unknown_command(format!("because f*** you, that's why. (command: {:?})", request.op).as_ref())),
        }
    }
}

/// A result type with the error type hard-coded to `tracker::Error`.
pub type Result<'a, T> = result::Result<T, Error>;

/// They types of error that might result from a tracker request.
#[derive(Debug)]
pub enum ErrorKind {
    UnknownCommand,
    IoError(io::Error),
    Other(String),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match *self {
            ErrorKind::UnknownCommand => "unknown_command",
            ErrorKind::IoError(_) => "io_error",
            ErrorKind::Other(ref s) => s.as_ref(),
        };
        write!(f, "{}", s)
    }
}

/// An error coming from handling a tracker request.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    description: String,
    // cause: Option<Box<error::Error>>,
}

impl Error {
    pub fn error_line(&self) -> String {
        let encoded_description = percent_encoding::percent_encode(
            self.description.as_bytes(),
            percent_encoding::FORM_URLENCODED_ENCODE_SET);
        format!("ERR {} {}", self.kind, encoded_description)
    }

    pub fn unknown_command(desc: &str) -> Error {
        Error {
            kind: ErrorKind::UnknownCommand,
            description: desc.to_string(),
            // cause: None,
        }
    }

    pub fn other(kind: &str, desc: &str) -> Error {
        Error {
            kind: ErrorKind::Other(kind.to_string()),
            description: desc.to_string(),
            // cause: None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ERR {} {}", self.kind, self.description)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.description
    }
}

impl From<io::Error> for Error {
    fn from(io_err: io::Error) -> Error {
        // Oy, the pain.
        use std::error::Error;
        self::Error {
            description: io_err.description().to_string(),
            kind: ErrorKind::IoError(io_err),
            // cause: Box::new(io_err),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use regex::Regex;
//     use std::io::{Cursor, Read};
//     use super::*;

//     #[test]
//     fn error_kinds() {
//         assert_eq!("unknown_command", format!("{}", ErrorKind::UnknownCommand));
//         assert_eq!("arbitrary_error", format!("{}", ErrorKind::Other("arbitrary_error")));
//     }

//     #[test]
//     fn error_line() {
//         let e = Error::unknown_command("unknown command: blah");
//         assert_eq!("ERR unknown_command unknown%20command%3A%20blah", e.error_line());
//     }

//     fn handler_fixture() -> Tracker {
//         Tracker::new()
//     }

//     #[test]
//     fn dispatch_unknown_command() {
//         let handler = handler_fixture();
//         let request = "this_command_doesnt_exist key1=val1&domain=foo";
//         let result = handler.dispatch_command(request);
//         assert!(result.is_err());
//         assert_eq!(ErrorKind::UnknownCommand, result.unwrap_err().kind);
//     }

//     #[test]
//     fn handle_unknown_command() {
//         let response_re = Regex::new("^ERR unknown_command [^ ]+\r\n").unwrap();
//         let handler = handler_fixture();
//         let request_bytes: Vec<u8> = "this_command_doesnt_exist key1=val1&domain=foo\r\n".bytes().collect();
//         let mut request = Cursor::new(request_bytes);
//         let mut response = vec![];
//         handler.handle(&mut request, &mut response);
//         assert!(response_re.is_match(String::from_utf8_lossy(&response).as_ref()));
//     }
// }
