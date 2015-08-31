use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::str;
use super::error::{MogError, MogResult};
use super::util::FromBytes;
use url::form_urlencoded;
use url::percent_encoding::{self, FORM_URLENCODED_ENCODE_SET};

/// The response from the tracker.
#[derive(Debug)]
pub struct Response(MogResult<HashMap<String, String>>);

impl Response {
    pub fn new_ok<F>(callback: F) -> Response
        where F: Fn(&mut HashMap<String, String>)
    {
        let mut args_hash = HashMap::new();
        callback(&mut args_hash);
        Response(Ok(args_hash))
    }

    pub fn new_err(err: MogError) -> Response {
        Response(Err(err))
    }

    pub fn as_hash(&self) -> Option<&HashMap<String, String>> {
        self.0.as_ref().ok()
    }

    pub fn as_mut_hash(&mut self) -> Option<&mut HashMap<String, String>> {
        self.0.as_mut().ok()
    }

    pub fn ok(self) -> Option<HashMap<String, String>> {
        self.0.ok()
    }

    pub fn err(self) -> Option<MogError> {
        self.0.err()
    }

    pub fn render(&self) -> Vec<u8> {
        match self.0 {
            Ok(ref args) => format!("OK {}\r\n", form_urlencoded::serialize(args)).into_bytes(),
            Err(ref err) => {
                let encoded_description = percent_encoding::percent_encode(
                    format!("{}", err).as_bytes(),
                    percent_encoding::FORM_URLENCODED_ENCODE_SET);
                format!("ERR {} {}\r\n", err.error_kind(), encoded_description).into_bytes()
            }
        }
    }
}

impl From<MogResult<Response>> for Response {
    fn from(result: MogResult<Response>) -> Response {
        match result {
            Ok(response) => response,
            Err(err) => Response(Err(err)),
        }
    }
}

impl FromBytes for Response {
    fn from_bytes(bytes: &[u8]) -> MogResult<Response> {
        let mut reader = BufReader::new(bytes);
        let mut code = Vec::new();
        let mut arg_bytes = Vec::new();

        try!(reader.read_until(b' ', &mut code));
        try!(reader.read_to_end(&mut arg_bytes));

        match str::from_utf8(&code) {
            Ok("OK ") => {
                Ok(Response::new_ok(|resp_hash| {
                    for (k, v) in form_urlencoded::parse(&arg_bytes).into_iter() {
                        resp_hash.entry(k).or_insert(v);
                    }
                }))
            },

            Ok("ERR ") => {
                Ok(Response(Err(MogError::from_bytes(&arg_bytes))))
            },

            Ok(e) => Err(MogError::UnknownCode(e.to_string())),
            Err(utf8e) => Err(MogError::Utf8(utf8e)),
        }
    }
}
