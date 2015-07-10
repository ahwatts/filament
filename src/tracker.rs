use std::collections::HashMap;
use std::convert::AsRef;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{BufRead, BufReader, Read, Write};
use std::result;
use url::{form_urlencoded, percent_encoding};

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    UnknownCommand,
    Other(String),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match *self {
            ErrorKind::UnknownCommand => "unknown_command",
            ErrorKind::Other(ref s) => s.as_ref(),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    description: String,
    cause: Option<Box<error::Error>>,
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
            cause: None,
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

pub type Result<'a, T> = result::Result<T, Error>;
type CommandArgs = HashMap<String, Vec<String>>;
type ResponseArgs = Vec<(String, String)>;

pub struct Handler;

impl Handler {
    pub fn new() -> Handler {
        Handler
    }
    
    pub fn handle<R: Read, W: Write>(&self, read_stream: &mut R, write_stream: &mut W) {
        let reader = BufReader::new(read_stream);

        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    println!("request  = {:?}", line);
                    let response = self.dispatch_command(&line.trim_right());
                    println!("response = {:?}", response);

                    // Okay, both arms here are the same, but maybe they
                    // won't be in the future?
                    match response {
                        Ok(response_args) => {
                            write!(write_stream, "{}\r\n", form_urlencoded::serialize(response_args))
                                .unwrap_or_else(|e| println!("Error writing successful response: {:?}", e));
                        },
                        Err(error) => {
                            write!(write_stream, "{}\r\n", error.error_line())
                                .unwrap_or_else(|e| println!("Error writing error response: {:?}", e));
                        }
                    }
                },
                Err(e) => {
                    println!("Error with connection: {:?}", e);
                    break;
                }
            }
        }
    }

    fn dispatch_command(&self, line: &str) -> Result<ResponseArgs> {
        let mut toks = line.split(" ");
        let command = toks.next();
        let args_str = toks.next();
        let args = parse_query_string(args_str.unwrap_or("").as_bytes());

        println!("parsed request: command = {:?} args = {:?}", command, args);

        match command {
            _ => Err(Error::unknown_command(format!("because f*** you, that's why. (command: {:?})", command).as_ref())),
        }
    }
}

fn parse_query_string(query_string: &[u8]) -> CommandArgs {
    let parsed = form_urlencoded::parse(query_string);
    parsed.into_iter().fold(HashMap::new(), |mut m, (k, v)| {
        m.entry(k).or_insert(vec![]).push(v); m
    })
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::io::{Cursor, Read};
    use super::*;

    #[test]
    fn error_kinds() {
        assert_eq!("unknown_command", format!("{}", ErrorKind::UnknownCommand));
        assert_eq!("arbitrary_error", format!("{}", ErrorKind::Other("arbitrary_error".to_string())));
    }

    #[test]
    fn error_line() {
        let e = Error::unknown_command("unknown command: blah");
        assert_eq!("ERR unknown_command unknown%20command%3A%20blah", e.error_line());
    }

    fn handler_fixture() -> Handler {
        Handler::new()
    }

    #[test]
    fn dispatch_unknown_command() {
        let handler = handler_fixture();
        let request = "this_command_doesnt_exist key1=val1&domain=foo";
        let result = handler.dispatch_command(request);
        assert!(result.is_err());
        assert_eq!(ErrorKind::UnknownCommand, result.unwrap_err().kind);
    }

    #[test]
    fn handle_unknown_command() {
        let response_re = Regex::new("^ERR unknown_command [^ ]+\r\n").unwrap();
        let handler = handler_fixture();
        let request_bytes: Vec<u8> = "this_command_doesnt_exist key1=val1&domain=foo\r\n".bytes().collect();
        let mut request = Cursor::new(request_bytes);
        let mut response = vec![];
        handler.handle(&mut request, &mut response);
        assert!(response_re.is_match(String::from_utf8_lossy(&response).as_ref()));
    }
}
