use std::collections::HashMap;
use super::args_hash::ArgsHash;
use super::error::MogResult;
use super::util::FromBytes;
use url::form_urlencoded;
use url::percent_encoding::{self, FORM_URLENCODED_ENCODE_SET};

/// The response from the tracker.
#[derive(Debug)]
pub struct Response(MogResult<HashMap<String, String>>);

impl Response {
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
        let args = ArgsHash::from_bytes(bytes);
        Ok(Response(Ok(args.as_hash().clone())))
    }
}
