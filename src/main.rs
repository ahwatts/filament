#![cfg_attr(test, allow(dead_code))]

extern crate docopt;
extern crate env_logger;
extern crate filament_ext;
extern crate iron;
extern crate libc;
extern crate mogilefs_common;
extern crate mogilefs_server;
extern crate rustc_serialize;
extern crate url;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

use docopt::Docopt;
use filament_ext::{MyOpts, AlternateFinderBackend, PublicFinder, SongFinder};
use iron::{Chain, Iron, Protocol};
use mogilefs_common::{BackendStack, AroundMiddleware};
use mogilefs_server::mem::{MemBackend, SyncMemBackend};
use mogilefs_server::net::storage::StorageHandler;
use mogilefs_server::net::tracker::Tracker;
use mogilefs_server::proxy::ProxyTrackerBackend;
use mogilefs_server::range::RangeMiddleware;
use rustc_serialize::{Decodable, Decoder};
use std::default::Default;
use std::net::SocketAddr;
use std::thread;
use url::Url;
use util::{SocketAddrList, WrapSocketAddr};

pub mod lookup;
pub mod util;

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

    let db_opts = opts.flag_db_host.as_ref().map(|addr| MyOpts {
        tcp_addr: match addr {
            &WrapSocketAddr(SocketAddr::V4(v4_ip)) => Some(v4_ip.ip().to_string()),
            &WrapSocketAddr(SocketAddr::V6(v6_ip)) => Some(v6_ip.ip().to_string()),
        },
        tcp_port: addr.0.port(),
        user: Some(opts.flag_db_user.clone()),
        pass: opts.flag_db_pass.clone(),
        db_name: Some(opts.flag_db_name.clone()),
        ..Default::default()
    });

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
            iron.listen_with(storage_addr, storage_threads, Protocol::Http, None).unwrap();
        });

        let mut tracker = Tracker::new(stack);
        if let Some(ref host) = opts.flag_statsd_host {
            if let Err(e) = tracker.report_stats_to(
                &format!("{}", host.0),
                opts.flag_statsd_prefix.as_ref().unwrap_or(&"".to_string())) {
                error!("Could not create statsd client: {}", e);
            }
        }

        Some(tracker)
    } else if opts.cmd_proxy_tracker {
        let backend = ProxyTrackerBackend::new(&opts.flag_real_trackers.0).unwrap();
        let mut stack = BackendStack::new(backend);

        if let Some(ref url) = opts.flag_alternate_base_url {
            info!("Retrieving alternate public images from {}", url);
            let public_finder = PublicFinder::new(url.clone());
            let pf_backend = AlternateFinderBackend::new(public_finder, db_opts.clone());
            stack.around(pf_backend);
        }

        if let Some(ref url) = opts.flag_alternate_song_api_url {
            info!("Retrieving alternate songs from {}", url);
            let song_finder = SongFinder::new(url.clone());
            let sf_backend = AlternateFinderBackend::new(song_finder, db_opts);
            stack.around(sf_backend);
        }

        let mut tracker = Tracker::new(stack);
        if let Some(ref host) = opts.flag_statsd_host {
            if let Err(e) = tracker.report_stats_to(
                &format!("{}", host.0),
                opts.flag_statsd_prefix.as_ref().unwrap_or(&"".to_string())) {
                error!("Could not create statsd client: {}", e);
            }
        }

        Some(tracker)
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
  --statsd-host=HOST         Report statistics to statsd here.
  --statsd-prefix=PREFIX     Prefix statsd statistic names with this.

General Tracker Options:
  --tracker-ip=IP            The ip:port for the tracker to listen on. [default: 0.0.0.0:7002]
  -t N, --tracker-threads=N  How many tracker threads to run.          [default: 4]
  -i T, --tracker-io=T       Which I/O model the tracker should use.   [default: Evented]
                             (can be Threaded or Evented)

General Storage Options:
  --storage-ip=IP            The ip:port for the storage server to listen on. [default: 0.0.0.0:7503]
  -s N, --storage-threads=N  How many storage threads to run.                 [default: 4]
  -u URL, --base-url=URL     The base URL for the storage server.             [default: http://127.0.0.1:7503/]

Database Options:
  (These can also be specified as environment variables prefixed by
  FILAMENT_, e.g. FILAMENT_DB_HOST, FILAMENT_DB_USER, etc.)
  --db-host=IP               The host ip:port to find the MogileFS DB on.
  --db-user=USER             The username to connect to the DB with.          [default: mogile]
  --db-pass=PASS             The password to connect to the DB with.
  --db-name=DB               The MogileFS database name.                      [default: mogilefs]


In-Memory Tracker (mem-tracker) Options:
  (all General Tracker Options and General Storage Options supported)

Proxy Tracker (proxy-tracker) Options:
  (all General Tracker Options and Database Options supported)
  Tracker Options:
    --real-trackers=IPS           A comma-separated list of actual trackers that we're proxying for.    [default: 127.0.0.1:7001]
    --alternate-base-url=URL      The base public URL to look for files that are missing on the real trackers.
    --alternate-song-api-url=URL  The base API URL to find missing song files.
";

#[derive(Debug, RustcDecodable)]
struct Options {
    cmd_mem_tracker: bool,
    cmd_proxy_tracker: bool,

    flag_statsd_host: Option<WrapSocketAddr>,
    flag_statsd_prefix: Option<String>,

    flag_tracker_ip: WrapSocketAddr,
    flag_tracker_threads: usize,
    flag_tracker_io: TrackerIoType,

    flag_storage_ip: WrapSocketAddr,
    flag_storage_threads: usize,
    flag_base_url: Url,

    flag_db_host: Option<WrapSocketAddr>,
    flag_db_user: String,
    flag_db_pass: Option<String>,
    flag_db_name: String,

    flag_real_trackers: SocketAddrList,
    flag_alternate_base_url: Option<Url>,
    flag_alternate_song_api_url: Option<Url>,
}

#[derive(Debug, RustcDecodable)]
enum TrackerIoType {
    Threaded,
    Evented,
}
