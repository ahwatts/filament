// #![allow(unused_variables)]

extern crate libc;
extern crate mio;
extern crate threadpool;
extern crate url;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[cfg(test)]
extern crate regex;

// macro_rules! debug {
//     (target: $target:expr, $($arg:tt)*) => ();
//     ($($arg:tt)*) => ()
// }

// macro_rules! info {
//     (target: $target:expr, $($arg:tt)*) => ();
//     ($($arg:tt)*) => ()
// }

// macro_rules! warn {
//     (target: $target:expr, $($arg:tt)*) => ();
//     ($($arg:tt)*) => ()
// }

// macro_rules! error {
//     (target: $target:expr, $($arg:tt)*) => ();
//     ($($arg:tt)*) => ()
// }

pub mod ctrlc;
pub mod tracker;
