extern crate chrono;
extern crate hyper;
extern crate iron;
extern crate libc;
extern crate mio;
extern crate mogilefs_client;
extern crate mogilefs_common;
extern crate mysql;
extern crate plugin;
extern crate r2d2;
extern crate statsd;
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
pub mod database;
pub mod mem;
pub mod monitor;
pub mod net;
pub mod proxy;
pub mod r2d2_statsd;
pub mod range;

#[cfg(test)]
pub mod test_support {
    pub use super::mem::test_support::*;
}
