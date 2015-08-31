extern crate docopt;
extern crate env_logger;
extern crate rustc_serialize;
extern crate mogilefs_client;
extern crate mogilefs_common;
extern crate url;

#[macro_use] extern crate log;

use docopt::Docopt;
use mogilefs_client::MogClient;
use mogilefs_common::requests::*;
use mogilefs_common::FromBytes;
use rustc_serialize::{Decodable, Decoder};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::option::IntoIter;

pub fn main() {
    env_logger::init().unwrap();

    let opts: Options = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    debug!("opts = {:?}", opts);

    let mut client = MogClient::new(&opts.arg_tracker);

    match opts.arg_command.as_ref() {
        "file_info" => {
            let args_str = opts.arg_args.expect("The file_info command requires arguments.");
            let req = FileInfo::from_bytes(args_str.as_bytes()).unwrap();
            let res = client.file_info(&req.domain, &req.key).unwrap();
            println!("{:?}", res);
        },
        _ => {
            println!("Unknown command: {:}", opts.arg_command);
        }
    }
}

static USAGE: &'static str = "
A command-line tool for querying a MogileFS system.

Usage:
  filament-cli t <tracker>... c <command> [<args>]

General Options:
  -h, --help                 Print this help message.
";

#[derive(Debug, RustcDecodable)]
enum TrackerIoType {
    Threaded,
    Evented,
}

#[derive(Debug, RustcDecodable)]
struct Options {
    arg_tracker: Vec<WrapSocketAddr>,
    arg_command: String,
    arg_args: Option<String>,
}

// Need to wrap SocketAddr with our own type so that we can implement
// RustcDecodable for it.
#[derive(Debug)]
struct WrapSocketAddr(SocketAddr);

impl Decodable for WrapSocketAddr {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use std::str::FromStr;
        let addr_str = try!(d.read_str());
        SocketAddr::from_str(&addr_str)
            .map(|a| WrapSocketAddr(a))
            .map_err(|e| d.error(format!("Error parsing address {:?}: {:?}",
                                         addr_str, e).as_ref()))
    }
}

impl ToSocketAddrs for WrapSocketAddr {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> io::Result<IntoIter<SocketAddr>> {
        self.0.to_socket_addrs()
    }
}
