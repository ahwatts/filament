use super::error::MogResult;
use url::form_urlencoded;
use url::percent_encoding::{self, FORM_URLENCODED_ENCODE_SET};

/// The response from the tracker.
#[derive(Debug)]
pub struct Response(MogResult<Vec<(String, String)>>);

impl Response {
    pub fn new(args: Vec<(String, String)>) -> Response {
        Response(Ok(args))
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
