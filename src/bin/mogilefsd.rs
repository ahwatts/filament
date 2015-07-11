#![cfg_attr(test, allow(dead_code))]

extern crate argparse;
extern crate mio;
extern crate mogilefsd;

use argparse::ArgumentParser;
use mio::{EventLoop, Handler, NonBlock, ReadHint, Token, Interest, PollOpt};
use mio::tcp::{self, TcpListener, TcpStream};
use mogilefsd::tracker;
use mogilefsd::listener::ListenerPool;
use std::default::Default;
use std::net::{ToSocketAddrs, Ipv4Addr};
use std::sync::Arc;
use std::thread;
use std::collections::HashMap;
use std::io::{Read, Write};

fn main() {
    let mut opts: Options = Default::default();
    opts.parser().parse_args_or_exit();

    let tracker = tracker::Handler::new();

    let sock_addr = (opts.listen_ip, opts.listen_port).to_socket_addrs().unwrap().next().unwrap();
    let server_token = Token(0);
    let server = tcp::listen(&sock_addr).unwrap_or_else(|e| {
        panic!("Error setting up listener on {:?}: {}", sock_addr, e);
    });

    let mut event_loop: EventLoop<MfsEventHandler> = EventLoop::new().unwrap();
    event_loop.register_opt(&server, server_token, Interest::all(), PollOpt::all()).unwrap();
    event_loop.run(&mut MfsEventHandler::new(server, server_token)).unwrap();

    // let tracker_listener = TcpListener::bind((opts.listen_ip, opts.listen_port)).unwrap();
    // let tracker_handler = Arc::new(tracker::Handler::new());
    // let mfs_supervisor_thread = thread::spawn(move|| {
    //     let pool = ListenerPool::new(tracker_listener);
    //     pool.accept(move|mut stream: TcpStream| {
    //         let mut read_stream = stream.try_clone().unwrap();
    //         println!(
    //             "Connection received: local = {:?} remote = {:?}",
    //             stream.local_addr(), stream.peer_addr());
    //         tracker_handler.clone().handle(&mut read_stream, &mut stream)
    //     }, 4);
    // });

    // mfs_supervisor_thread.join().unwrap_or_else(|e| {
    //     println!("MogileFSd supervisor thread panicked: {:?}", e);
    // })
}

struct MfsEventHandler {
    server: NonBlock<TcpListener>,
    tracker: tracker::Handler,
    server_token: Token,
    conns: HashMap<Token, NonBlock<TcpStream>>,
    last_token: Token,
}

impl MfsEventHandler {
    pub fn new(server: NonBlock<TcpListener>, server_token: Token) -> MfsEventHandler {
        MfsEventHandler {
            server: server,
            tracker: tracker::Handler::new(),
            server_token: server_token,
            conns: HashMap::new(),
            last_token: server_token,
        }
    }
}

impl Handler for MfsEventHandler {
    type Timeout = usize;
    type Message = usize;

    fn readable(&mut self, event_loop: &mut EventLoop<Self>, token: Token, hint: ReadHint) {
        // println!("Readable event! server: {:?} token: {:?} hint: {:?}", self.server, token, hint);
        match token {
            t if t == self.server_token => {
                match self.server.accept() {
                    Ok(Some(stream)) => {
                        let conn_token = Token(self.last_token.as_usize() + 1);
                        let register_result = event_loop.register_opt(
                            &stream, conn_token,
                            Interest::readable(),
                            PollOpt::level());

                        match  register_result {
                            Ok(_) => {
                                println!(
                                    "Connection received: local = {:?} remote = {:?} token = {:?}",
                                    stream.local_addr(), stream.peer_addr(), conn_token);
                                self.conns.insert(conn_token, stream);
                                self.last_token = conn_token;
                            },
                            Err(e) => {
                                println!("Error registering connection: {}", e);
                            }
                        }
                    },
                    Ok(None) => {
                        println!("Connection is not ready.");
                    },
                    Err(e) => {
                        println!("Connection failed: {}", e);
                    }
                }
            },
            t if self.conns.contains_key(&t) => {
                let conn = self.conns.get_mut(&t).unwrap();
                let mut s = String::new();
                // let read_result = conn.read_to_string(&mut s);

                match conn.read_to_string(&mut s) {
                    Ok(0) => {},
                    Ok(n) => {
                        println!("Readable event for connection {:?} bytes_read = {:?} s = {:?}", t, n, s);
                    },
                    Err(e) => {
                        println!("Error reading from stream for connection {:?}: {}", t, e);
                    }
                }
            },
            _ => {
                println!("Readable event! server: {:?} token: {:?} hint: {:?}", self.server, token, hint);
            }
        }
    }

    fn writable(&mut self, _: &mut EventLoop<Self>, token: Token) {
        // println!("Writeable event! server: {:?} token: {:?}", self.server, token);

        match token {
            t if self.conns.contains_key(&t) => {
                let conn = self.conns.get_mut(&t).unwrap();
                match conn.write_all(b"test") {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Error writing to stream for connection {:?}: {}", t, e);
                    }
                }
            }
            _ => {
                println!("Writeable event! server: {:?} token: {:?}", self.server, token);
            }
        }
    }

    fn notify(&mut self, _: &mut EventLoop<Self>, msg: Self::Message) {
        println!("Notify event! server: {:?} message: {:?}", self.server, msg);
    }

    fn interrupted(&mut self, _: &mut EventLoop<Self>) {
        println!("Interrupted event! server: {:?}", self.server);
    }
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
