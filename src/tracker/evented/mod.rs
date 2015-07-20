use mio::buf::{Buf, RingBuf};
use mio::tcp::{self, TcpListener, TcpStream};
use mio::{self, EventLoop, Interest, NonBlock, PollOpt, ReadHint, Socket, Token, TryRead, TryWrite};
use self::notification::Notification;
use self::tracker_pool::TrackerPool;
use std::collections::HashMap;
use std::net::{Shutdown, ToSocketAddrs};
use std::rc::Rc;
use super::{Request, Response};
use super::Tracker;
use super::super::ctrlc::CtrlC;

pub use self::error::{EventedError, EventedResult};

pub mod error;
mod notification;
mod tracker_pool;

pub struct EventedListener {
    event_loop: EventLoop<Handler>,
    handler: Handler,
}

impl EventedListener {
    pub fn new<T>(addr: T, tracker: Tracker, threads: usize) -> EventedResult<EventedListener>
        where T: ToSocketAddrs
    {
        Ok(EventedListener {
            event_loop: try!(EventLoop::new()),
            handler: try!(Handler::new(addr, TrackerPool::new(tracker, threads))),
        })
    }

    pub fn run(&mut self) -> EventedResult<()> {
        try!{
            self.event_loop.register_opt(
                &self.handler.server, self.handler.token,
                Interest::all(), PollOpt::edge())
        }

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
    server: NonBlock<TcpListener>,
    token: Token,
    conns: HashMap<Token, Connection>,
    last_token: Token,
    tracker: Rc<TrackerPool>,
}

impl Handler {
    pub fn new<T: ToSocketAddrs>(sock_addr: T, pool: TrackerPool) -> EventedResult<Handler> {
        let sock_addr = try!(try!(sock_addr.to_socket_addrs()).next().ok_or(EventedError::NoListenAddr));
        let token = Token(0);

        let socket = try!(tcp::v4());
        try!(socket.set_reuseaddr(true));
        try!(socket.bind(&sock_addr));
        let server = try!(socket.listen(256));

        let handler = Handler {
            server: server,
            token: token,
            conns: HashMap::new(),
            last_token: token,
            tracker: Rc::new(pool),
        };

        Ok(handler)
    }

    fn accept(&mut self, event_loop: &mut EventLoop<Self>) -> EventedResult<()> {
        let stream = try!(try!(self.server.accept()).ok_or(EventedError::StreamNotReady));
        let conn = Connection::new(stream, Token(self.last_token.as_usize() + 1), self.tracker.clone());
        info!("New connection {:?} from {:?}", conn.token, conn.sock.peer_addr());
        debug!("Registering {:?} as {:?} / edge", conn.token, conn.interest);
        try!(event_loop.register_opt(&conn.sock, conn.token, conn.interest, PollOpt::edge()));
        self.last_token = conn.token;
        self.conns.insert(conn.token, conn);
        Ok(())
    }

    fn shutdown(&mut self, event_loop: &mut EventLoop<Self>) {
        let keys: Vec<Token> = self.conns.keys().cloned().collect();

        for t in keys.iter() {
            self.close_connection(event_loop, *t).unwrap_or_else(|e| {
                warn!("Error closing down connection {:?}: {}", t, e);
            });
        }

        event_loop.shutdown();
    }

    fn close_connection(&mut self, event_loop: &mut EventLoop<Self>, token: Token) -> EventedResult<()> {
        match self.conns.remove(&token) {
            Some(conn) => conn.shutdown(event_loop),
            None => Err(EventedError::UnknownConnection(token)),
        }
    }

    fn write_response(&mut self, event_loop: &mut EventLoop<Self>, token: Token, response: Response) -> EventedResult<()> {
        match self.conns.get_mut(&token) {
            Some(conn) => conn.write_response(event_loop, response),
            None => Err(EventedError::UnknownConnection(token)),
        }
    }
}

impl mio::Handler for Handler {
    type Timeout = usize;
    type Message = Notification;

    fn readable(&mut self, event_loop: &mut EventLoop<Self>, token: Token, hint: ReadHint) {
        match token {
            t if t == self.token => {
                self.accept(event_loop).unwrap_or_else(|e| {
                    error!("Error accepting connection: {} (hint: {:?})", e, hint);
                });
            },
            t if self.conns.contains_key(&t) => {
                let conn = self.conns.get_mut(&t).unwrap();
                conn.readable(event_loop, hint).unwrap_or_else(|e| {
                    error!("Error handling readable event for connection {:?}: {}", t, e);
                });
            },
            _ => {
                warn!("Readable event for unknown connection {:?}", token);
            }
        }
    }

    fn writable(&mut self, event_loop: &mut EventLoop<Self>, token: Token) {
        match token {
            t if self.conns.contains_key(&t) => {
                let conn = self.conns.get_mut(&t).unwrap();
                conn.writable(event_loop).unwrap_or_else(|e| {
                    error!("Error handling writable event for connection {:?}: {}", t, e);
                });
            }
            _ => {
                warn!("Writable event for unknown connection: {:?}", token);
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, message: Notification) {
        debug!("Notify event: message = {:?}", message);
        match message {
            Notification::Shutdown => self.shutdown(event_loop),
            Notification::CloseConnection(token) => {
                self.close_connection(event_loop, token).unwrap_or_else(|e| {
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
    sock: NonBlock<TcpStream>,
    token: Token,
    in_buf: RingBuf,
    out_buf: RingBuf,
    interest: Interest,
    tracker: Rc<TrackerPool>,
}

impl Connection {
    pub fn new(sock: NonBlock<TcpStream>, token: Token, tracker: Rc<TrackerPool>) -> Connection {
        Connection {
            sock: sock,
            token: token,
            in_buf: RingBuf::new(2048),
            out_buf: RingBuf::new(2048),
            interest: Interest::readable() | Interest::hup() | Interest::error(),
            tracker: tracker,
        }
    }

    fn readable(&mut self, event_loop: &mut EventLoop<Handler>, hint: ReadHint) -> EventedResult<()> {
        let read_result = self.sock.read(&mut self.in_buf);

        match read_result {
            Ok(Some(n)) => {
                debug!("Read {} bytes from {:?}", n, self.token);
            },
            Ok(None) => {
                debug!("No more bytes to read from {:?}", self.token);
            },
            Err(ref e) => {
                debug!("Error reading from {:?}: {}", self.token, e);
            }
        }

        match self.extract_line(&self.in_buf) {
            Some(line) => {
                Buf::advance(&mut self.in_buf, line.len() + 2);
                self.tracker.handle(Request::from(line.as_ref()), self.token, event_loop.channel());
                // self.interest = Interest::writable() | Interest::hup() | Interest::error();
                // debug!("Registering {:?} as {:?} / edge", self.token, self.interest);
                // try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
            },
            None => {}
        }

        if hint.is_hup() | hint.is_error() {
            try!(event_loop.channel().send(Notification::close_connection(self.token)));
        }

        read_result.map(|_| ()).map_err(|e| EventedError::from(e))
    }

    fn writable(&mut self, event_loop: &mut EventLoop<Handler>) -> EventedResult<()> {
        let write_result = self.sock.write(&mut self.out_buf);

        match write_result {
            Ok(Some(n)) => {
                debug!("Wrote {} bytes to {:?}", n, self.token);
            },
            Ok(None) => {
                debug!("Not ready to write to {:?}", self.token);
            },
            Err(ref e) => {
                error!("Error writing to {:?}: {}", self.token, e);
            }
        }

        // We should probably only switch back to readable if we've
        // written a newline, but for now we'll just assume that the
        // response gets written to the buffer in toto.
        if Buf::has_remaining(&self.out_buf) {
            self.interest = Interest::writable() | Interest::hup() | Interest::error();
            debug!("Registering {:?} as {:?} / edge", self.token, self.interest);
            try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
        } else {
            self.interest = Interest::readable() | Interest::hup() | Interest::error();
            debug!("Registering {:?} as {:?} / edge", self.token, self.interest);
            try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
        }

        write_result.map(|_| ()).map_err(|e| EventedError::from(e))
    }

    fn write_response(&mut self, event_loop: &mut EventLoop<Handler>, response: Response) -> EventedResult<()> {
        use std::io::Write;

        try!(self.out_buf.write(&response.render()));
        self.interest = Interest::writable() | Interest::hup() | Interest::error();
        debug!("Registering {:?} as {:?} / edge", self.token, self.interest);
        try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
        Ok(())
    }

    fn shutdown(self, event_loop: &mut EventLoop<Handler>) -> EventedResult<()> {
        use std::io::Write;

        info!("Shutting down {:?} from {:?}", self.token, self.sock.peer_addr());
        debug!("Deregistering {:?}", self.token);
        try!(event_loop.deregister(&self.sock));

        let mut unwrapped = self.sock.unwrap();
        try!(unwrapped.flush());
        try!(unwrapped.shutdown(Shutdown::Both));

        Ok(())
    }

    fn extract_line<T: Buf>(&self, buf: &T) -> Option<Vec<u8>> {
        let bytes = buf.bytes();
        let mut line = None;

        for (i, w) in bytes.windows(2).enumerate() {
            if w == &[ b'\r', b'\n' ] {
                line = Some(Vec::from(&bytes[0..i]));
                break;
            }
        }

        line
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Write, BufRead, BufReader};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::thread::{self, JoinHandle};
    use super::*;
    use super::notification::Notification;
    use super::super::Tracker;
    use super::super::super::common::test_support::*;

    fn fixture_server() -> EventedListener {
        let tracker = Tracker::new(sync_backend_fixture());
        EventedListener::new("0.0.0.0:0", tracker, 1).unwrap()
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
        let server_addr = server.handler.server.local_addr().unwrap();
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
        let server_addr = server.handler.server.local_addr().unwrap();
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
}
