extern crate hyper;
extern crate rand;
extern crate url;

#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate matches;

pub use backend::{Backend, BackendStack, AroundMiddleware};
pub use error::{MogError, MogResult};
pub use request::{Request, Response, ToResponse, Renderable};
pub use util::{BufReadMb, FromBytes, ToArgs, ToUrlencodedString};

/// The specific request / response types, in a separate module for
/// easy globbing.
pub mod requests {
    pub use request::CreateDomain;
    pub use request::{CreateOpen, CreateOpenResponse};
    pub use request::CreateClose;
    pub use request::{CreateClass, CreateClassResponse};
    pub use request::{GetPaths, GetPathsResponse};
    pub use request::{FileInfo, FileInfoResponse};
    pub use request::Rename;
    pub use request::UpdateClass;
    pub use request::Delete;
    pub use request::{ListKeys, ListKeysResponse};
    pub use request::Noop;
}

mod args_hash;
mod backend;
mod error;
mod request;
mod util;

#[cfg(test)]
pub mod test_support {
    pub use backend::test_support::*;
}
