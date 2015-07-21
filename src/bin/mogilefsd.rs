#![cfg_attr(test, allow(dead_code))]

extern crate argparse;
extern crate env_logger;
extern crate iron;
extern crate mogilefsd;
extern crate url;

#[macro_use]
extern crate log;

use argparse::ArgumentParser;
use iron::{Chain, Iron, Protocol};
use mogilefsd::common::{Backend, SyncBackend};
use mogilefsd::tracker::Tracker;
use mogilefsd::storage::Storage;
use mogilefsd::storage::iron::StorageHandler;
use std::default::Default;
use std::net::Ipv4Addr;
use std::thread;
use url::Url;

fn main() {
    env_logger::init().unwrap();

    let mut opts: Options = Default::default();
    opts.parser().parse_args_or_exit();

    let backend = SyncBackend::new(Backend::new());
    let tracker = Tracker::new(backend.clone());
    let storage = Storage::new(backend.clone(), opts.storage_base_url.clone());

    let storage_addr = opts.storage_addr();
    let storage_threads = opts.storage_threads;
    thread::spawn(move|| {
        let iron = Iron::new(Chain::new(StorageHandler::new(storage)));
        iron.listen_with(storage_addr, storage_threads, Protocol::Http).unwrap();
    });

    run(&opts, tracker);
}

#[cfg(feature = "evented")]
fn run(opts: &Options, tracker: Tracker) {
    use mogilefsd::tracker::evented::EventedListener;

    let listener_result = EventedListener::new(
        opts.tracker_addr(),
        tracker,
        opts.tracker_threads);

    let mut listener = listener_result.unwrap_or_else(|e| {
        panic!("Error creating evented listener on {:?}: {}", opts.tracker_addr(), e);
    });

    listener.run().unwrap_or_else(|e| {
        panic!("Error running evented listener: {}", e);
    });
}

#[cfg(not(feature = "evented"))]
fn run(opts: &Options, tracker: Tracker) {
    use mogilefsd::tracker::threaded::ThreadedListener;

    let listener_result = ThreadedListener::new(
        opts.tracker_addr(),
        tracker);

    let listener = listener_result.unwrap_or_else(|e| {
        panic!("Error creating threaded listener on {:?}: {}", opts.tracker_addr(), e);
    });

    listener.run();
}

#[derive(Debug)]
struct Options {
    tracker_ip: Ipv4Addr,
    tracker_port: u16,
    tracker_threads: usize,

    storage_ip: Ipv4Addr,
    storage_port: u16,
    storage_threads: usize,
    storage_base_url: Url,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            tracker_ip: Ipv4Addr::new(0, 0, 0, 0),
            tracker_port: 7002,
            tracker_threads: 4,

            storage_ip: Ipv4Addr::new(0, 0, 0, 0),
            storage_port: 7502,
            storage_threads: 4,
            storage_base_url: Url::parse("http://127.0.0.1:7502").unwrap(),
        }
    }
}

impl Options {
    fn parser(&mut self) -> ArgumentParser {
        let mut parser = ArgumentParser::new();
        parser.set_description("A partial clone for the MogileFS tracker daemon.");

        parser.refer(&mut self.tracker_ip).add_option(
            &[ "--tracker-ip" ],
            argparse::Store,
            "The host IP for the tracker to listen on.");

        parser.refer(&mut self.tracker_port).add_option(
            &[ "--tracker-port" ],
            argparse::Store,
            "The port for the tracker to listen on.");

        parser.refer(&mut self.tracker_threads).add_option(
            &[ "--tracker-threads" ],
            argparse::Store,
            "How many threads the tracker should run.");

        parser.refer(&mut self.storage_ip).add_option(
            &[ "--storage-ip" ],
            argparse::Store,
            "The host IP for the storage server to listen on.");

        parser.refer(&mut self.storage_port).add_option(
            &[ "--storage-port" ],
            argparse::Store,
            "The port for the storage server to listen on.");

        parser.refer(&mut self.storage_threads).add_option(
            &[ "--storage-threads" ],
            argparse::Store,
            "How many threads the storage server should run.");

        parser.refer(&mut self.storage_base_url).add_option(
            &[ "--storage-base-url" ],
            argparse::Store,
            "The base URL that the storage server should report.");

        parser
    }

    fn tracker_addr(&self) -> (Ipv4Addr, u16) {
        (self.tracker_ip, self.tracker_port)
    }

    fn storage_addr(&self) -> (Ipv4Addr, u16) {
        (self.storage_ip, self.storage_port)
    }
}
