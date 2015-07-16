use std::collections::HashMap;
use std::error::Error;
use super::{TrackerError, TrackerResult};
use url::{form_urlencoded, percent_encoding};

/// A request to or response from a MogileFS tracker.
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

impl From<TrackerError> for Message {
    fn from(err: TrackerError) -> Message {
        Message {
            op: format!("{}", err.kind),
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
