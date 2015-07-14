extern crate libc;
extern crate mio;
extern crate url;

#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate regex;

pub mod ctrlc;
pub mod tracker;
pub mod evserver;
