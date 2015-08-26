extern crate log;
extern crate url;

#[cfg(test)]
#[macro_use]
extern crate matches;

pub use error::{MogError, MogResult};
pub use request::Request;
pub use response::Response;
pub use request::types as requests;

mod error;
mod response;
mod request;
