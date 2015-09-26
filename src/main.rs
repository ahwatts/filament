#![cfg_attr(test, allow(dead_code))]

extern crate docopt;
extern crate env_logger;
extern crate filament_ext;
extern crate iron;
extern crate mogilefs_common;
extern crate mogilefs_server;
extern crate rustc_serialize;
extern crate url;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

use docopt::Docopt;
use filament_ext::{PublicFinder, SongFinder};
use iron::{Chain, Iron, Protocol};
use mogilefs_common::{BackendStack, AroundMiddleware};
use mogilefs_server::mem::{MemBackend, SyncMemBackend};
use mogilefs_server::net::storage::StorageHandler;
use mogilefs_server::net::tracker::Tracker;
use mogilefs_server::proxy::ProxyTrackerBackend;
use mogilefs_server::range::RangeMiddleware;
use rustc_serialize::{Decodable, Decoder};
use std::net::SocketAddr;
use std::thread;
use url::Url;

static VERSION_NUM: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
static GIT_COMMIT: &'static str = include_str!("../git-revision");

lazy_static!{
    static ref FULL_VERSION: String =
        format!("filament version {} commit {}",
                VERSION_NUM.unwrap_or("unknown"), GIT_COMMIT);
}

fn main() {
    env_logger::init().unwrap();

    let opts: Options = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(FULL_VERSION.to_string())).decode())
        .unwrap_or_else(|e| e.exit());
    debug!("opts = {:?}", opts);

    let tracker = if opts.cmd_mem_tracker {
        let backend = SyncMemBackend::new(MemBackend::new(opts.flag_base_url.clone()));
        let stack = BackendStack::new(backend.clone());

        let storage_addr = opts.flag_storage_ip.0.clone();
        let storage_threads = opts.flag_storage_threads;
        thread::spawn(move|| {
            let mut chain = Chain::new(StorageHandler::new(backend));
            chain.around(RangeMiddleware);
            let iron = Iron::new(chain);
            println!("Storage server (Iron) listening on {:?}", storage_addr);
            iron.listen_with(storage_addr, storage_threads, Protocol::Http).unwrap();
        });

        Some(Tracker::new(stack))
    } else if opts.cmd_proxy_tracker {
        let backend = ProxyTrackerBackend::new(&opts.flag_real_trackers.0).unwrap();
        let mut stack = BackendStack::new(backend);

        if opts.flag_alternate_base_url.is_some() {
            let public_finder = PublicFinder::new(opts.flag_alternate_base_url.as_ref().cloned().unwrap());
            stack.around(public_finder);
        }

        if opts.flag_alternate_song_api_url.is_some() {
            let song_finder = SongFinder::new(opts.flag_alternate_song_api_url.as_ref().cloned().unwrap());
            stack.around(song_finder);
        }

        Some(Tracker::new(stack))
    } else {
        None
    };

    match (tracker, &opts.flag_tracker_io) {
        (Some(tracker), &TrackerIoType::Evented) => run_evented(&opts, tracker),
        (Some(tracker), &TrackerIoType::Threaded) => run_threaded(&opts, tracker),
        _ => panic!("Don't know how to run the tracker!"),
    }
}

fn run_evented(opts: &Options, tracker: Tracker<BackendStack>) {
    use mogilefs_server::net::tracker::evented::EventedListener;

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

fn run_threaded(opts: &Options, tracker: Tracker<BackendStack>) {
    use mogilefs_server::net::tracker::threaded::ThreadedListener;

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
  filament (-h | --help | -v | --version)
  filament mem-tracker [options]
  filament proxy-tracker [options]

General Options:
  -h, --help                 Print this help message.
  -v, --version              Print the version information.

General Tracker Options:
  --tracker-ip=IP            The ip:port for the tracker to listen on. [default: 0.0.0.0:7002]
  -t N, --tracker-threads=N  How many tracker threads to run.          [default: 4]
  -i T, --tracker-io=T       Which I/O model the tracker should use.   [default: Evented]
                             (can be Threaded or Evented)

General Storage Options:
  --storage-ip=IP            The ip:port for the storage server to listen on. [default: 0.0.0.0:7503]
  -s N, --storage-threads=N  How many storage threads to run.                 [default: 4]
  -u URL, --base-url=URL     The base URL for the storage server.             [default: http://127.0.0.1:7503/]


In-Memory Tracker (mem-tracker) Options:
  (all General Tracker Options and General Storage Options supported)

Proxy Tracker (proxy-tracker) Options:
  (all General Tracker Options and General Storage Options supported)
  Tracker Options:
    --real-trackers=IPS           A comma-separated list of actual trackers that we're proxying for.    [default: 127.0.0.1:7001]
    --alternate-base-url=URL      The base public URL to look for files that are missing on the real trackers.
    --alternate-song-api-url=URL  The base API URL to find missing song files.
";

#[derive(Debug, RustcDecodable)]
struct Options {
    cmd_mem_tracker: bool,
    cmd_proxy_tracker: bool,

    flag_tracker_ip: WrapSocketAddr,
    flag_tracker_threads: usize,
    flag_tracker_io: TrackerIoType,

    flag_storage_ip: WrapSocketAddr,
    flag_storage_threads: usize,
    flag_base_url: Url,

    flag_real_trackers: SocketAddrList,
    flag_alternate_base_url: Option<Url>,
    flag_alternate_song_api_url: Option<Url>,
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
