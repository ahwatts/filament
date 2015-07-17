#![cfg_attr(test, allow(dead_code))]

extern crate argparse;
extern crate env_logger;
extern crate iron;
extern crate mogilefsd;
extern crate url;

#[macro_use]
extern crate log;

use argparse::ArgumentParser;
use iron::{Chain, Iron};
use mogilefsd::common::Backend;
use mogilefsd::tracker::Tracker;
use mogilefsd::storage::Storage;
use mogilefsd::storage::iron::StorageHandler;
use std::collections::HashMap;
use std::default::Default;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::thread;
use url::Url;

fn main() {
    env_logger::init().unwrap();

    let mut opts: Options = Default::default();
    opts.parser().parse_args_or_exit();

    let backend = Arc::new(Mutex::new(Backend(HashMap::new())));
    let tracker = Tracker::new(backend.clone());
    let storage = Storage::new(backend.clone(), Url::parse("http://127.0.0.1:7503").unwrap());

    thread::spawn(move|| {
        Iron::new(Chain::new(StorageHandler::new(storage))).http("127.0.0.1:7503").unwrap();
    });

    run(&opts, tracker);
}

#[cfg(feature = "evented")]
fn run(opts: &Options, tracker: Tracker) {
    use mogilefsd::tracker::evented::EventedListener;

    let listener_result = EventedListener::new(
        opts.listen_addr(),
        tracker,
        opts.tracker_threads);

    let mut listener = listener_result.unwrap_or_else(|e| {
        panic!("Error creating evented listener on {:?}: {}", opts.listen_addr(), e);
    });

    listener.run().unwrap_or_else(|e| {
        panic!("Error running evented listener: {}", e);
    });
}

#[cfg(not(feature = "evented"))]
fn run(opts: &Options, tracker: Tracker) {
    use mogilefsd::tracker::threaded::ThreadedListener;

    let listener_result = ThreadedListener::new(
        opts.listen_addr(),
        tracker);

    let listener = listener_result.unwrap_or_else(|e| {
        panic!("Error creating threaded listener on {:?}: {}", opts.listen_addr(), e);
    });

    listener.run();
}

#[derive(Debug)]
struct Options {
    listen_ip: Ipv4Addr,
    listen_port: u16,
    tracker_threads: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            listen_ip: Ipv4Addr::new(0, 0, 0, 0),
            listen_port: 7002,
            tracker_threads: 4,
        }
    }
}

impl Options {
    fn parser(&mut self) -> ArgumentParser {
        let mut parser = ArgumentParser::new();
        parser.set_description("A partial clone for the MogileFS tracker daemon.");

        parser.refer(&mut self.listen_ip).add_option(
            &[ "-l", "--listen-ip" ],
            argparse::Store,
            "The host IP for the tracker to listen on.");

        parser.refer(&mut self.listen_port).add_option(
            &[ "-p", "--listen-port" ],
            argparse::Store,
            "The port for the tracker to listen on.");

        parser.refer(&mut self.tracker_threads).add_option(
            &[ "-t", "--tracker-threads" ],
            argparse::Store,
            "How many threads the tracker should run.");

        parser
    }

    fn listen_addr(&self) -> (Ipv4Addr, u16) {
        (self.listen_ip, self.listen_port)
    }
}
