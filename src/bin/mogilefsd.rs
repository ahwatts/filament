#![cfg_attr(test, allow(dead_code))]

extern crate argparse;
extern crate env_logger;
extern crate mio;
extern crate mogilefsd;

#[macro_use]
extern crate log;

use argparse::ArgumentParser;
use mogilefsd::tpool::TrackerPool;
use mogilefsd::evserver::{Server, ServerHandler};
use mogilefsd::tracker::Tracker;
use std::default::Default;
use std::net::Ipv4Addr;

fn main() {
    env_logger::init().unwrap();

    let mut opts: Options = Default::default();
    opts.parser().parse_args_or_exit();
    let listen_addr = (opts.listen_ip, opts.listen_port);

    let tracker = Tracker::new();
    let pool = TrackerPool::new(tracker, 4);

    let mut handler = ServerHandler::new(listen_addr, pool).unwrap_or_else(|e| {
        panic!("Error setting up server on {:?}: {}", listen_addr, e);
    });

    let mut server = Server::new().unwrap_or_else(|e| {
        panic!("Error creating event loop: {}", e);
    });

    server.run(&mut handler).unwrap_or_else(|e| {
        panic!("Error running event loop: {}", e);
    });
}

#[derive(Debug)]
struct Options {
    listen_ip: Ipv4Addr,
    listen_port: u16,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            listen_ip: Ipv4Addr::new(0, 0, 0, 0),
            listen_port: 7002,
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
            &[ "-l", "--listen-ip" ],
            argparse::Store,
            "The port for the tracker to listen on.");

        parser
    }
}
