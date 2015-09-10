extern crate iron;
extern crate libc;
extern crate mio;
extern crate mogilefs_client;
extern crate mogilefs_common;
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
pub mod ctrlc;
pub mod mem;
pub mod net;
pub mod proxy;
pub mod range;

#[cfg(test)]
pub mod test_support {
    pub use super::mem::test_support::*;
}
