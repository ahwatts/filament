use mio::tcp::{Shutdown, TcpListener, TcpStream};
use mio::util::Slab;
use mio::{self, EventLoop, EventSet, PollOpt, Token, TryRead, TryWrite};
use self::notification::Notification;
use self::tracker_pool::TrackerPool;
use std::io::{self, BufRead, BufReader, Cursor, Read, Write};
use std::net::ToSocketAddrs;
use std::rc::Rc;
use super::super::ctrlc::CtrlC;
use super::{Tracker, Response};

pub use self::error::{EventedError, EventedResult};

pub mod error;
mod notification;
mod tracker_pool;

static CRLF: &'static [u8] = &[ b'\r', b'\n' ];

lazy_static!{
    static ref READABLE: EventSet = EventSet::readable() | EventSet::hup() | EventSet::error();
    static ref WRITABLE: EventSet = EventSet::writable() | EventSet::hup() | EventSet::error();
    static ref EDGE_ONESHOT: PollOpt = PollOpt::edge() | PollOpt::oneshot();
}

pub struct EventedListener {
    event_loop: EventLoop<Handler>,
    handler: Handler,
}

impl EventedListener {
    pub fn new<T>(addr: T, tracker: Tracker, max_conns: usize, threads: usize) -> EventedResult<EventedListener>
        where T: ToSocketAddrs
    {
        Ok(EventedListener {
            event_loop: try!(EventLoop::new()),
            handler: try!(Handler::new(addr, max_conns, TrackerPool::new(tracker, threads))),
        })
    }

    pub fn run(&mut self) -> EventedResult<()> {
        // Register the server socket with the event loop.
        try!(self.event_loop.register(&self.handler.listener, self.handler.token));

        // register a handler for ctrl+c.
        let notify_channel = self.event_loop.channel();
        CtrlC::set_handler(move|| {
            notify_channel.send(Notification::shutdown()).unwrap_or_else(|e| {
                error!("Error notifying event loop of SIGINT: {:?}", e);
            });
        });

        Ok(try!(self.event_loop.run(&mut self.handler)))
    }
}

struct Handler {
    listener: TcpListener,
    token: Token,
    conns: Slab<Connection>,
    tracker: Rc<TrackerPool>,
}

impl Handler {
    pub fn new<T: ToSocketAddrs>(sock_addr: T, max_conns: usize, pool: TrackerPool) -> EventedResult<Handler> {
        let sock_addr = try!(try!(sock_addr.to_socket_addrs()).next().ok_or(EventedError::NoListenAddr));
        let token = Token(0);
        let listener = try!(TcpListener::bind(&sock_addr));

        let handler = Handler {
            listener: listener,
            token: token,
            conns: Slab::new_starting_at(Token(1), max_conns),
            tracker: Rc::new(pool),
        };

        Ok(handler)
    }

    fn shutdown(&mut self, event_loop: &mut EventLoop<Self>) {
        let tokens: Vec<Token> = self.conns.iter().map(|c| c.token.clone()).collect();

        for &t in tokens.iter() {
            self.close(event_loop, t).unwrap_or_else(|e| {
                warn!("Error closing down connection {:?}: {}", t, e);
            });
        }

        event_loop.shutdown();
    }

    fn accept(&mut self, event_loop: &mut EventLoop<Self>) -> EventedResult<()> {
        match self.listener.accept() {
            Ok(Some(stream)) => {
                let tracker = self.tracker.clone();
                self.conns
                    .insert_with(|token| Connection::new(stream, token, tracker))
                    .ok_or(EventedError::TooManyConnections)
                    .and_then(|token| {
                        info!("New connection {:?} from {:?}", token, &self.conns[token].stream.peer_addr());
                        debug!("Registering {:?} as {:?} / {:?}", token, *READABLE, *EDGE_ONESHOT);
                        event_loop.register_opt(
                            &self.conns[token].stream, token,
                            *READABLE, *EDGE_ONESHOT)
                            .map_err(|e| EventedError::from(e))
                    })
            },
            Ok(None) => {
                Err(EventedError::StreamNotReady)
            },
            Err(e) => {
                Err(EventedError::from(e))
            }
        }
    }

    fn close(&mut self, event_loop: &mut EventLoop<Self>, token: Token) -> EventedResult<()> {
        match self.conns.remove(token) {
            Some(conn) => conn.shutdown(event_loop),
            None => Err(EventedError::UnknownConnection(token)),
        }
    }

    fn write_response(&mut self, event_loop: &mut EventLoop<Self>, token: Token, response: Response) -> EventedResult<()> {
        match self.conns.get_mut(token) {
            Some(conn) => {
                let result = conn.write_response(event_loop, response);

                debug!("Re-registering {:?} as {:?} / {:?}", token, *WRITABLE, *EDGE_ONESHOT);
                event_loop.reregister(&conn.stream, conn.token, *WRITABLE, *EDGE_ONESHOT).unwrap_or_else(|e|{
                    error!("Error re-registering {:?} as {:?}: {}", conn.token, *WRITABLE, e);
                });

                result
            }
            None => Err(EventedError::UnknownConnection(token)),
        }
    }
}

impl mio::Handler for Handler {
    type Timeout = usize;
    type Message = Notification;

    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        debug!("Ready event for connection {:?}: {:?}", token, events);

        match token {
            t if t == self.token => {
                if events.is_readable() {
                    self.accept(event_loop).unwrap_or_else(|e| {
                        error!("Error accepting connection: {}", e);
                    });
                } else {
                    error!("Unknown event type {:?} on server socket.", events);
                }
            },
            t if self.conns.contains(t) => {
                let mut reregister_as = *READABLE;
                let ref mut conn = self.conns[token];

                if events.is_readable() {
                    reregister_as = conn.read(event_loop).unwrap_or_else(|e| {
                        error!("Error handling readable event for conection {:?}: {}", token, e);
                        *READABLE
                    });
                }

                if events.is_writable() {
                    reregister_as = conn.write(event_loop).unwrap_or_else(|e| {
                        error!("Error handling writable event for conection {:?}: {}", token, e);
                        *WRITABLE
                    });
                }

                if events.is_error() || events.is_hup() {
                    debug!("Notifying event loop of closed / errored connection {:?}: {:?}", token, events);
                    event_loop.channel().send(Notification::close_connection(token)).unwrap_or_else(|e| {
                        error!("Error notifying event loop of closed / errored conection {:?}: {}",
                               token, EventedError::from(e));
                    });
                } else {
                    debug!("Re-registering {:?} as {:?} / {:?}", token, reregister_as, *EDGE_ONESHOT);
                    event_loop.reregister(&conn.stream, conn.token, reregister_as, *EDGE_ONESHOT).unwrap_or_else(|e|{
                        error!("Error re-registering {:?} as {:?}: {}", conn.token, reregister_as, e);
                    });
                }
            },
            _ => {
                warn!("Ready event for unknown connection: {:?}", token);
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, message: Notification) {
        debug!("Notify event: message = {:?}", message);
        match message {
            Notification::Shutdown => self.shutdown(event_loop),
            Notification::CloseConnection(token) => {
                self.close(event_loop, token).unwrap_or_else(|e| {
                    error!("Error closing connection {:?}: {}", token, e);
                });
            },
            Notification::Response(token, response) => {
                self.write_response(event_loop, token, response).unwrap_or_else(|e| {
                    error!("Error writing tracker response to {:?}: {}", token, e);
                });
            }
        }
    }

    // fn timeout(&mut self, _: &mut EventLoop<Self>, timeout: usize) {
    //     debug!("Timeout event: timeout = {:?}", timeout);
    // }

    fn interrupted(&mut self, event_loop: &mut EventLoop<Self>) {
        debug!("Interrupted event.");
        event_loop.channel().send(Notification::shutdown()).unwrap_or_else(|e| {
            error!("Error handling interrupted event by sending message: {:?}", e);
        });
    }
}

struct Connection {
    stream: TcpStream,
    token: Token,
    in_buf: Vec<u8>,
    out_buf: Vec<u8>,
    tracker: Rc<TrackerPool>,
    current: Option<Vec<u8>>,
}

impl Connection {
    pub fn new(stream: TcpStream, token: Token, tracker: Rc<TrackerPool>) -> Connection {
        Connection {
            stream: stream,
            token: token,
            in_buf: Vec::new(),
            out_buf: Vec::new(),
            tracker: tracker,
            current: None,
        }
    }

    fn read(&mut self, event_loop: &mut EventLoop<Handler>) -> EventedResult<EventSet> {
        match self.stream.try_read_buf(&mut self.in_buf) {
            Ok(Some(n)) => {
                debug!("Read {} bytes from {:?}", n, self.token);
                self.maybe_dispatch_request(event_loop);
                Ok(*READABLE)
            },
            Ok(None) => {
                debug!("No more bytes to read from {:?}", self.token);
                Ok(*READABLE)
            },
            Err(e) => {
                Err(EventedError::from(e))
            }
        }
    }

    fn write(&mut self, _event_loop: &mut EventLoop<Handler>) -> EventedResult<EventSet> {
        let wrote_bytes = {
            // Wrap this in a separate scope so that we can modify
            // out_buf later.
            let mut reader = Cursor::new(self.out_buf.as_ref());
            match self.stream.try_write_buf(&mut reader) {
                Ok(Some(n)) if n == self.out_buf.len() => {
                    debug!("Wrote {} bytes (the whole output buffer) to {:?}",
                           n, self.token);
                    Ok(n)
                },
                Ok(Some(n)) => {
                    debug!("Wrote {} bytes to {:?}; {} bytes still need to be written",
                           n, self.token, self.out_buf.len() - n);
                    Ok(n)
                },
                Ok(None) => {
                    debug!("Not ready to write to {:?}", self.token);
                    Ok(0)
                },
                Err(e) => {
                    Err(EventedError::from(e))
                }
            }
        };

        match wrote_bytes {
            Ok(0) => Ok(*WRITABLE),
            Ok(n) if n == self.out_buf.len() => {
                self.out_buf.clear();
                Ok(*READABLE)
            },
            Ok(n) => {
                let rest = Vec::from(self.out_buf[n..self.out_buf.len()].as_ref());
                self.out_buf = rest;
                Ok(*WRITABLE)
            },
            Err(e) => Err(e),
        }
    }

    fn maybe_dispatch_request(&mut self, event_loop: &mut EventLoop<Handler>) {
        debug!("in_buf = {:?} ({:?})", self.in_buf, String::from_utf8_lossy(&self.in_buf));

        if self.current.is_none() && self.in_buf.windows(2).position(|x| x == CRLF).is_some() {
            let mut request = vec![];
            let mut rest = vec![];

            {
                // There should be no way this can go wrong; we're reading
                // from and writing to Vecs.
                let mut reader = BufReader::new(Cursor::new(self.in_buf.as_ref()));
                read_until_mb(&mut reader, CRLF, &mut request).unwrap();
                reader.read_to_end(&mut rest).unwrap();
            }

            debug!("request = {:?} rest = {:?}",
                   String::from_utf8_lossy(&request),
                   String::from_utf8_lossy(&rest));

            self.current = Some(request.clone());
            self.in_buf = rest;
            self.tracker.handle(request, self.token, event_loop.channel());
        }
    }

    fn write_response(&mut self, event_loop: &mut EventLoop<Handler>, response: Response) -> EventedResult<()> {
        self.out_buf.write(&response.render()).unwrap();
        self.current = None;
        self.maybe_dispatch_request(event_loop);
        Ok(())
    }

    fn shutdown(mut self, event_loop: &mut EventLoop<Handler>) -> EventedResult<()> {
        info!("Shutting down {:?} from {:?}", self.token, self.stream.peer_addr());
        debug!("Deregistering {:?}", self.token);
        try!(event_loop.deregister(&self.stream));

        try!(self.stream.flush());
        try!(self.stream.shutdown(Shutdown::Both));

        Ok(())
    }
}

/// A version of the standard library's read_until() function that
/// supports a multibyte delimiter.
fn read_until_mb<R: BufRead + ?Sized>(r: &mut R, delim: &[u8], buf: &mut Vec<u8>) -> io::Result<usize> {
    use std::io::ErrorKind;

    let mut read = 0;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e)
            };
            match available.windows(delim.len()).position(|x| x == delim) {
                Some(i) => {
                    buf.extend(&available[..i + delim.len()]);
                    (true, i + delim.len())
                }
                None => {
                    buf.extend(available);
                    (false, available.len())
                }
            }
        };
        r.consume(used);
        read += used;
        if done || used == 0 {
            return Ok(read);
        }
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::io::{Write, BufRead, BufReader};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::thread::{self, JoinHandle};
    use super::*;
    use super::notification::Notification;
    use super::super::Tracker;
    use super::super::super::storage::Storage;
    use super::super::super::common::test_support::*;
    use url::Url;

    fn fixture_server() -> EventedListener {
        let backend = sync_backend_fixture();
        let storage = Storage::new(backend.clone(), Url::parse("http://127.0.0.1:12345").unwrap());
        let tracker = Tracker::new(backend, storage.clone());
        EventedListener::new("0.0.0.0:0", tracker, 1, 1).unwrap()
    }

    fn client_thread<S: ToSocketAddrs, F>(addr: S, func: F) -> JoinHandle<()>
        where F: FnOnce(BufReader<TcpStream>, TcpStream) + Send + 'static
    {
        let server_addr = addr.to_socket_addrs().unwrap().next().unwrap();

        thread::spawn(move|| {
            let writer = TcpStream::connect(server_addr).unwrap();
            let reader = BufReader::new(writer.try_clone().unwrap());
            func(reader, writer);
        })
    }

    #[test]
    fn basic_interaction() {
        let mut server = fixture_server();
        let server_addr = server.handler.listener.local_addr().unwrap();
        let channel = server.event_loop.channel();

        let handle = client_thread(server_addr, move|mut reader, mut writer| {
            let mut resp = String::new();
            assert!(resp.is_empty());

            writer.write("file_info domain=rn_development_private&key=test/key/2\r\n".as_bytes()).unwrap();
            reader.read_line(&mut resp).unwrap();
            assert!(!resp.is_empty());

            resp.clear();
            assert!(resp.is_empty());

            writer.write("file_info domain=rn_development_private&key=test/key/3\r\n".as_bytes()).unwrap();
            reader.read_line(&mut resp).unwrap();
            assert!(!resp.is_empty());

            channel.send(Notification::shutdown()).unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn oneshot_reading() {
        let mut server = fixture_server();
        let server_addr = server.handler.listener.local_addr().unwrap();
        let channel = server.event_loop.channel();

        let handle = client_thread(server_addr, move|mut reader, mut writer| {
            let mut resp = String::new();
            assert!(resp.is_empty());

            writer.write("file_info domain=rn_develop".as_bytes()).unwrap();
            thread::sleep_ms(1000);
            writer.write("ment_private&key=test/key/2\r\n".as_bytes()).unwrap();

            reader.read_line(&mut resp).unwrap();
            assert!(!resp.is_empty());


            channel.send(Notification::shutdown()).unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn multiple_requests_in_a_write() {
        let mut server = fixture_server();
        let server_addr = server.handler.listener.local_addr().unwrap();
        let channel = server.event_loop.channel();

        let handle = client_thread(server_addr, move|mut reader, mut writer| {
            let mut resp = String::new();
            assert!(resp.is_empty());

            writer.write("list_keys domain=test_domain_1\r\ncreate_domain domain=test_domain_2\r\n".as_bytes()).unwrap();

            reader.read_line(&mut resp).unwrap();
            let re = Regex::new("^OK .*key_count=").unwrap();
            assert!(re.is_match(&resp));
            resp.clear();

            reader.read_line(&mut resp).unwrap();
            let re = Regex::new("^OK .*domain=").unwrap();
            assert!(re.is_match(&resp));
            resp.clear();

            channel.send(Notification::shutdown()).unwrap();
        });

        server.run().unwrap();
        handle.join().unwrap();
    }
}
