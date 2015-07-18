use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str;
use super::{TrackerError, TrackerErrorKind, TrackerResult};
use url::{form_urlencoded, percent_encoding};

#[derive(Debug)]
pub enum Command {
    CreateOpen,
    Noop,

    // This enum also includes errors on their way out.
    UnknownCommand,
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::Command::*;

        let op_str = match *self {
            CreateOpen => "create_open",
            Noop => "noop",
            UnknownCommand => "unknown_command",
        };

        write!(f, "{}", op_str)
    }
}

impl<'a> From<&'a str> for Command {
    fn from(string: &'a str) -> Command {
        use self::Command::*;
        match string {
            "create_open" => CreateOpen,
            _ => Noop,
        }
    }
}

impl<'a> From<Option<&'a [u8]>> for Command {
    fn from(bytes: Option<&'a [u8]>) -> Command {
        Command::from(str::from_utf8(bytes.unwrap_or(b"")).unwrap_or(""))
    }
}

impl<'a> From<&'a TrackerErrorKind> for Command {
    fn from(kind: &'a TrackerErrorKind) -> Command {
        match *kind {
            TrackerErrorKind::UnknownCommand => Command::UnknownCommand,
            _ => Command::Noop,
        }
    }
}

/// A request to or response from a MogileFS tracker.
#[derive(Debug)]
pub struct Message {
    pub op: Command,
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
        let command = Command::from(toks.next());
        let parsed_args = form_urlencoded::parse(toks.next().unwrap_or(b""));

        let body = if parsed_args.len() == 1 && parsed_args[0].1 == "" {
            MessageBody::Message(parsed_args[0].0.clone())
        } else {
            MessageBody::Args(parsed_args)
        };

        Message {
            op: command,
            body: body,
        }
    }
}

impl From<TrackerError> for Message {
    fn from(err: TrackerError) -> Message {
        Message {
            op: Command::from(&err.kind),
            body: MessageBody::Message(err.description().to_string()),
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
    pub fn render(&self) -> String {
        match *self {
            MessageBody::Args(ref args) => form_urlencoded::serialize(args),
            MessageBody::Message(ref msg) => percent_encoding::percent_encode(
                msg.as_bytes(), percent_encoding::FORM_URLENCODED_ENCODE_SET),
        }
    }
}

/// Something that can be turned in to a MogileFS request or response.
pub trait ToMessage {
    fn to_message<'a>(self) -> TrackerResult<'a, Message>;
}

impl ToMessage for Message {
    fn to_message<'a>(self) -> TrackerResult<'a, Message> {
        Ok(self)
    }
}

impl<'b> ToMessage for &'b [u8] {
    fn to_message<'a>(self) -> TrackerResult<'a, Message> {
        Ok(Message::from(self))
    }
}
