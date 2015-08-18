#![cfg_attr(test, allow(dead_code))]

extern crate docopt;
extern crate env_logger;
extern crate iron;
extern crate mogilefsd;
extern crate rustc_serialize;
extern crate url;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

use docopt::Docopt;
use iron::{Chain, Iron, Protocol};
use mogilefsd::mem::{MemBackend, SyncMemBackend, MemStorage};
use mogilefsd::net::tracker::Tracker;
use mogilefsd::net::storage::StorageHandler;
use rustc_serialize::{Decodable, Decoder};
use std::net::SocketAddr;
use std::thread;
use url::Url;

static VERSION_NUM: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
static GIT_COMMIT: &'static str = include_str!("../../git-revision");

lazy_static!{
    static ref FULL_VERSION: String =
        format!("mogilefsd-rs version {} commit {}",
                VERSION_NUM.unwrap_or("unknown"), GIT_COMMIT);
}

fn main() {
    env_logger::init().unwrap();

    let opts: Options = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(FULL_VERSION.to_string())).decode())
        .unwrap_or_else(|e| e.exit());
    debug!("opts = {:?}", opts);

    let backend = SyncMemBackend::new(MemBackend::new());
    let storage = MemStorage::new(backend.clone(), opts.flag_base_url.clone());
    let tracker = Tracker::new(backend.clone(), storage.clone());

    let storage_addr = opts.flag_storage_ip.0.clone();
    let storage_threads = opts.flag_storage_threads;
    thread::spawn(move|| {
        let iron = Iron::new(Chain::new(StorageHandler::new(storage)));
        println!("Storage server (Iron) listening on {:?}", storage_addr);
        iron.listen_with(storage_addr, storage_threads, Protocol::Http).unwrap();
    });

    match opts.flag_tracker_io {
        TrackerIoType::Evented => run_evented(&opts, tracker),
        TrackerIoType::Threaded => run_threaded(&opts, tracker),
    }
}

fn run_evented(opts: &Options, tracker: Tracker) {
    use mogilefsd::net::tracker::evented::EventedListener;

    let listener_result = EventedListener::new(
        opts.flag_tracker_ip.0,
        tracker,
        1024, opts.flag_tracker_threads);

    let mut listener = listener_result.unwrap_or_else(|e| {
        panic!("Error creating evented listener on {:?}: {}", opts.flag_tracker_ip.0, e);
    });

    println!("Tracker (evented) listening on {:?}", opts.flag_tracker_ip.0);
    listener.run().unwrap_or_else(|e| {
        panic!("Error running evented listener: {}", e);
     });
}

fn run_threaded(opts: &Options, tracker: Tracker) {
    use mogilefsd::net::tracker::threaded::ThreadedListener;

    let listener_result = ThreadedListener::new(
        opts.flag_tracker_ip.0,
        tracker);

    let listener = listener_result.unwrap_or_else(|e| {
        panic!("Error creating threaded listener on {:?}: {}", opts.flag_tracker_ip.0, e);
    });

    println!("Tracker (threaded) listening on {:?}", opts.flag_tracker_ip.0);
    listener.run();
}

static USAGE: &'static str = "
A quasi-workalike for the MogileFS tracker daemon.

Usage:
  mogilefsd [options]

General Options:
  -h, --help                 Print this help message.
  -v, --version              Print the version information.

Tracker Options:
  --tracker-ip=IP            The ip:port for the tracker to listen on. [default: 0.0.0.0:7002]
  -t N, --tracker-threads=N  How many tracker threads to run.          [default: 4]
  -i T, --tracker-io=T       Which I/O model the tracker should use.   [default: Threaded]
                             (can be Threaded or Evented)

Storage Options:
  --storage-ip=IP            The ip:port for the storage server to listen on. [default: 0.0.0.0:7503]
  -s N, --storage-threads=N  How many storage threads to run.                 [default: 4]
  -u URL, --base-url=URL     The base URL for the storage server.             [default: http://127.0.0.1:7503/]
";

#[derive(Debug, RustcDecodable)]
struct Options {
    flag_tracker_ip: WrapSocketAddr,
    flag_tracker_threads: usize,
    flag_tracker_io: TrackerIoType,

    flag_storage_ip: WrapSocketAddr,
    flag_storage_threads: usize,
    flag_base_url: Url,
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

#[derive(Debug, RustcDecodable)]
enum TrackerIoType {
    Threaded,
    Evented,
}
