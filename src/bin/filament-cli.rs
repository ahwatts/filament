extern crate docopt;
extern crate env_logger;
extern crate rustc_serialize;
extern crate mogilefs_client;
extern crate mogilefs_common;
extern crate url;

#[macro_use] extern crate log;

use docopt::Docopt;
use mogilefs_client::MogClient;
use rustc_serialize::{Decodable, Decoder};
use std::net::SocketAddr;

pub fn main() {
    env_logger::init().unwrap();

    let opts: Options = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    debug!("opts = {:?}", opts);

    let mut client = MogClient::new(opts.flag_trackers.as_slice());

    if opts.cmd_file_info {
        let domain = opts.arg_domain.expect("No domain provided.");
        let key = opts.arg_key.expect("No key provided.");
        let resp_rslt = client.file_info(&domain, &key);

        match resp_rslt {
            Ok(resp) => println!("{:?}", resp),
            Err(e) => println!("Error: {}", e),
        }
    } else {
        println!("No command provided?!");
    }
}

static USAGE: &'static str = "
A command-line tool for querying a MogileFS system.

Usage:
  filament-cli [options] file-info <domain> <key>
  filament-cli (-h | --help)

General Options:
  -h, --help                 This help message.
  -t IPS, --trackers IPS     A comma-separated list of tracker ip:port combinations [default: 127.0.0.1:7001]
";

#[derive(Debug, RustcDecodable)]
struct Options {
    flag_trackers: SocketAddrList,

    arg_domain: Option<String>,
    arg_key: Option<String>,
    cmd_file_info: bool,
}

#[derive(Debug)]
pub struct SocketAddrList(Vec<SocketAddr>);

impl SocketAddrList {
    pub fn as_slice(&self) -> &[SocketAddr] {
        &self.0
    }
}

impl Decodable for SocketAddrList {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use std::str::FromStr;

        let addrs_str = try!(d.read_str());
        let mut addrs = Vec::new();

        for addr_str in addrs_str.split(',') {
            let addr = try!(SocketAddr::from_str(addr_str).map_err(|e| d.error(&format!("Unable to parse address {:?}: {:?}", addr_str, e))));
            addrs.push(addr);
        }

        Ok(SocketAddrList(addrs))
    }
}
