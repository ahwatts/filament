extern crate log;
extern crate url;

#[cfg(test)]
#[macro_use]
extern crate matches;

pub use error::{MogError, MogResult};
pub use request::Request;
pub use response::Response;

mod error;
mod response;

pub mod request;
