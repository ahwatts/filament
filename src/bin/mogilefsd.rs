#![cfg_attr(test, allow(dead_code))]

extern crate argparse;
extern crate mio;
extern crate mogilefsd;

use argparse::ArgumentParser;
use mio::buf::{Buf, ByteBuf, MutByteBuf};
use mio::tcp::{self, TcpListener, TcpStream};
use mio::{EventLoop, Handler, NonBlock, ReadHint, Token, Interest, PollOpt, TryRead, TryWrite};
// use mogilefsd::listener::ListenerPool;
// use mogilefsd::tracker;
use std::collections::HashMap;
use std::default::Default;
use std::io::{Read, Write};
use std::net::{ToSocketAddrs, Ipv4Addr};
use std::iter;

fn main() {
    let mut opts: Options = Default::default();
    opts.parser().parse_args_or_exit();

    // let tracker = tracker::Handler::new();

    let sock_addr = (opts.listen_ip, opts.listen_port).to_socket_addrs().unwrap().next().unwrap();
    let server_token = Token(0);
    let server = tcp::listen(&sock_addr).unwrap_or_else(|e| {
        panic!("Error setting up listener on {:?}: {}", sock_addr, e);
    });

    let mut event_loop: EventLoop<MfsEventHandler> = EventLoop::new().unwrap();
    event_loop.register_opt(
        &server, server_token,
        Interest::all(),
        PollOpt::edge())
        .unwrap();
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

struct MfsConnection {
    sock: NonBlock<TcpStream>,
    token: Token,
    buf: Option<ByteBuf>,
    mut_buf: Option<MutByteBuf>,
    interest: Interest,
}

struct MfsEventHandler {
    server: NonBlock<TcpListener>,
    // tracker: tracker::Handler,
    server_token: Token,
    conns: HashMap<Token, MfsConnection>,
    last_token: Token,
}

impl MfsEventHandler {
    pub fn new(server: NonBlock<TcpListener>, server_token: Token) -> MfsEventHandler {
        MfsEventHandler {
            server: server,
            // tracker: tracker::Handler::new(),
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
                        let conn = MfsConnection {
                            sock: stream,
                            token: Token(self.last_token.as_usize() + 1),
                            buf: None,
                            mut_buf: Some(ByteBuf::mut_with_capacity(2048)),
                            interest: Interest::readable(),
                        };

                        match event_loop.register_opt(&conn.sock, conn.token, conn.interest, PollOpt::edge()) {
                            Ok(_) => {
                                println!(
                                    "Connection received: local = {:?} remote = {:?} token = {:?}",
                                    conn.sock.local_addr(), conn.sock.peer_addr(), conn.token);

                                self.last_token = conn.token;
                                self.conns.insert(conn.token, conn);
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
                let mut conn = self.conns.get_mut(&t).unwrap();
                let mut buf = conn.mut_buf.take().unwrap();

                match conn.sock.read(&mut buf) {
                    Ok(None) => {
                        println!("Socket is readable, but unable to read from it? token = {:?}", conn.token);
                    },
                    Ok(Some(n)) => {
                        println!("Readable event for connection {:?} bytes_read = {:?}", t, n);
                    },
                    Err(e) => {
                        println!("Error reading from stream for connection {:?}: {}", t, e);
                    }
                }

                let mut rbuf = buf.flip();
                let mut readout: Vec<u8> = iter::repeat(0u8).take(rbuf.capacity()).collect();
                let mut line = None;

                rbuf.mark();
                rbuf.read_slice(&mut readout);
                rbuf.reset();

                for (i, w) in readout.windows(2).enumerate() {
                    if w == &[ '\r' as u8, '\n' as u8 ] {
                        line = Some(String::from_utf8_lossy(&readout[0..i]));
                        break;
                    }
                }

                println!("line = {:?}", line);

                match line {
                    Some(_) => {
                        let mut wbuf = rbuf.flip();
                        wbuf.clear();
                        conn.buf = Some(wbuf.flip());
                        conn.interest = Interest::writable();
                        event_loop.reregister(&conn.sock, conn.token, conn.interest, PollOpt::edge()).unwrap();
                    },
                    None => {
                        conn.mut_buf = Some(rbuf.resume());
                    }
                }
            },

            _ => {
                println!("Readable event! server: {:?} token: {:?} hint: {:?}", self.server, token, hint);
            }
        }
    }

    fn writable(&mut self, event_loop: &mut EventLoop<Self>, token: Token) {
        match token {
            t if self.conns.contains_key(&t) => {
                let mut conn = self.conns.get_mut(&t).unwrap();
                let buf = conn.buf.take().unwrap();
                let mut wbuf = buf.flip();
                wbuf.write(b"ERR unknown_command unknown+command\r\n").unwrap();
                let mut rbuf = wbuf.flip();

                match conn.sock.write(&mut rbuf) {
                    Ok(n) => {
                        println!("Wrote {:?} bytes to {:?}", n, token);
                    },
                    Err(e) => {
                        println!("Error writing to stream for connection {:?}: {}", t, e);
                    }
                }

                let mut wbuf2 = rbuf.flip();
                wbuf2.clear();
                conn.mut_buf = Some(wbuf2);
                conn.interest = Interest::readable();
                event_loop.reregister(&conn.sock, conn.token, conn.interest, PollOpt::edge()).unwrap();
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
