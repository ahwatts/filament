extern crate iron;
extern crate libc;
extern crate mio;
extern crate rand;
extern crate threadpool;
extern crate time;
extern crate url;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

#[cfg(test)]
#[macro_use]
extern crate matches;

#[cfg(test)]
extern crate regex;

pub mod backend;
pub mod client;
pub mod ctrlc;
pub mod error;
pub mod mem;
pub mod net;
pub mod proxy;

#[cfg(test)]
pub mod test_support {
    pub use super::mem::test_support::*;
}
