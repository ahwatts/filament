use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str;
use super::{TrackerError};
use url::{form_urlencoded, percent_encoding};

/// The different commands that the tracker implements.
#[derive(Debug)]
pub enum Command {
    CreateOpen,
    Noop,
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::Command::*;

        let op_str = match *self {
            CreateOpen => "create_open",
            Noop => "noop",
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

/// A request to the MogileFS tracker.
#[derive(Debug)]
pub struct Request {
    pub op: Command,
    pub args: Vec<(String, String)>,
}

impl Request {
    pub fn args_hash<'a>(&'a self) -> HashMap<&'a str, &'a str> {
        self.args.iter().fold(HashMap::new(), |mut m, &(ref k, ref v)| {
            *m.entry(k).or_insert(v) = v; m
        })
    }
}

impl<'a> From<&'a [u8]> for Request {
    fn from(bytes: &[u8]) -> Request {
        let mut toks = bytes.split(|&c| c == b' ');
        Request {
            op: Command::from(toks.next()),
            args: form_urlencoded::parse(toks.next().unwrap_or(b"")),
        }
    }
}

/// The response from the tracker.
#[derive(Debug)]
pub enum Response {
    Ok(Vec<(String, String)>),
    Err(TrackerError),
}

impl Response {
    pub fn render(&self) -> Vec<u8> {
        use self::Response::*;

        match *self {
            Ok(ref args) => format!("OK {}\r\n", form_urlencoded::serialize(args)).into_bytes(),
            Err(ref err) => {
                let encoded_description = percent_encoding::percent_encode(
                    err.description().as_bytes(),
                    percent_encoding::FORM_URLENCODED_ENCODE_SET);
                format!("ERR {} {}\r\n", err.kind, encoded_description).into_bytes()
            }
        }
    }
}

impl From<TrackerError> for Response {
    fn from(err: TrackerError) -> Response {
        Response::Err(err)
    }
}
