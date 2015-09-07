extern crate log;
extern crate url;

#[cfg(test)]
#[macro_use]
extern crate matches;

pub use error::{MogError, MogResult};
pub use request::{Request, Response, Renderable};
// pub use request::types as requests;
// pub use response::Response;
pub use util::{BufReadMb, FromBytes, ToArgs, ToUrlencodedString};

/// The specific request types, in a separate module for easy
/// globbing.
pub mod requests {
    pub use request::CreateDomain;
    pub use request::{CreateOpen, CreateOpenResponse};
}

mod args_hash;
mod error;
// mod response;
mod request;
mod util;
