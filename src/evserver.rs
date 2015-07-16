use mio::buf::{Buf, RingBuf};
use mio::tcp::{self, TcpListener, TcpStream};
use mio::{EventLoop, Handler, Interest, NonBlock, NotifyError, PollOpt, ReadHint, Socket, Token, TryRead, TryWrite};
use std::collections::HashMap;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::net::{Shutdown, ToSocketAddrs};
use std::rc::Rc;
use std::result;
use super::ctrlc::CtrlC;
use super::tpool::TrackerPool;
use super::tracker::Message;

pub struct Server {
    event_loop: EventLoop<ServerHandler>,
}

impl Server {
    pub fn new() -> Result<Server> {
        Ok(Server {
            event_loop: try!(EventLoop::new()),
        })
    }

    pub fn run(&mut self, handler: &mut ServerHandler) -> Result<()> {
        try!{
            self.event_loop.register_opt(
                &handler.server, handler.token,
                Interest::all(), PollOpt::edge())
        }

        // register a handler for ctrl+c.
        let notify_channel = self.event_loop.channel();
        CtrlC::set_handler(move|| {
            notify_channel.send(Notification::shutdown()).unwrap_or_else(|e| {
                error!("Error notifying event loop of SIGINT: {:?}", e);
            });
        });

        Ok(try!(self.event_loop.run(handler)))
    }
}

pub struct ServerHandler {
    server: NonBlock<TcpListener>,
    token: Token,
    conns: HashMap<Token, Connection>,
    last_token: Token,
    tracker: Rc<TrackerPool>,
}

impl ServerHandler {
    pub fn new<T: ToSocketAddrs>(sock_addr: T, pool: TrackerPool) -> Result<ServerHandler> {
        let sock_addr = try!(try!(sock_addr.to_socket_addrs()).next().ok_or(Error::NoListenAddr));
        let token = Token(0);

        let socket = try!(tcp::v4());
        try!(socket.set_reuseaddr(true));
        try!(socket.bind(&sock_addr));
        let server = try!(socket.listen(256));

        let handler = ServerHandler {
            server: server,
            token: token,
            conns: HashMap::new(),
            last_token: token,
            tracker: Rc::new(pool),
        };

        Ok(handler)
    }

    fn accept(&mut self, event_loop: &mut EventLoop<Self>) -> Result<()> {
        let stream = try!(try!(self.server.accept()).ok_or(Error::StreamNotReady));
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

    fn close_connection(&mut self, event_loop: &mut EventLoop<Self>, token: Token) -> Result<()> {
        match self.conns.remove(&token) {
            Some(conn) => conn.shutdown(event_loop),
            None => Err(Error::UnknownConnection(token)),
        }
    }

    fn write_response(&mut self, event_loop: &mut EventLoop<Self>, token: Token, response: Message) -> Result<()> {
        match self.conns.get_mut(&token) {
            Some(conn) => conn.write_response(event_loop, response),
            None => Err(Error::UnknownConnection(token)),
        }
    }
}

impl Handler for ServerHandler {
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

#[derive(Debug)]
pub enum Notification {
    CloseConnection(Token),
    Shutdown,
    Response(Token, Message),
}

impl Notification {
    pub fn close_connection(token: Token) -> Notification {
        Notification::CloseConnection(token)
    }

    pub fn shutdown() -> Notification {
        Notification::Shutdown
    }
}

pub struct Connection {
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

    fn readable(&mut self, event_loop: &mut EventLoop<ServerHandler>, hint: ReadHint) -> Result<()> {
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
                self.tracker.handle(line.as_ref(), self.token, event_loop.channel());
                // self.interest = Interest::writable() | Interest::hup() | Interest::error();
                // debug!("Registering {:?} as {:?} / edge", self.token, self.interest);
                // try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
            },
            None => {}
        }

        if hint.is_hup() | hint.is_error() {
            try!(event_loop.channel().send(Notification::close_connection(self.token)));
        }

        read_result.map(|_| ()).map_err(|e| Error::from(e))
    }

    fn writable(&mut self, event_loop: &mut EventLoop<ServerHandler>) -> Result<()> {
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

        write_result.map(|_| ()).map_err(|e| Error::from(e))
    }

    fn write_response(&mut self, event_loop: &mut EventLoop<ServerHandler>, response: Message) -> Result<()> {
        use std::io::Write;

        try!(self.out_buf.write(response.render().as_bytes()));
        self.interest = Interest::writable() | Interest::hup() | Interest::error();
        debug!("Registering {:?} as {:?} / edge", self.token, self.interest);
        try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
        Ok(())
    }

    fn shutdown(self, event_loop: &mut EventLoop<ServerHandler>) -> Result<()> {
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

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    FullNotifyQueue,
    NoListenAddr,
    StreamNotReady,
    UnknownConnection(Token),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(ref io_err) => io_err.description(),
            Error::FullNotifyQueue => "Notification queue is full",
            Error::NoListenAddr => "Unable to determine address on which to listen",
            Error::StreamNotReady => "Stream is not ready",
            Error::UnknownConnection(_) => "Unknown connection"
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use std::error::Error;

        match *self {
            self::Error::IoError(ref io_err) => write!(f, "{}", io_err),
            self::Error::UnknownConnection(ref token) => write!(f, "Unknown connection: {:?}", token),
            _ => f.write_str(self.description()),
        }
    }
}

impl From<io::Error> for Error {
    fn from(io_err: io::Error) -> Error {
        Error::IoError(io_err)
    }
}

impl From<NotifyError<Notification>> for Error {
    fn from(not_err: NotifyError<Notification>) -> Error {
        match not_err {
            NotifyError::Io(io_err) => Error::IoError(io_err),
            NotifyError::Full(_) => Error::FullNotifyQueue,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use std::io::{self, Write, BufRead, BufReader};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::thread::{self, JoinHandle};
    use super::*;
    use super::super::tpool::TrackerPool;
    use super::super::tracker::Tracker;

    fn fixture_server() -> (Server, ServerHandler) {
        let tracker = Tracker::new();
        let pool = TrackerPool::new(tracker, 4);
        (Server::new().unwrap(), ServerHandler::new("0.0.0.0:0", pool).unwrap())
    }

    fn client_thread<S: ToSocketAddrs, F>(addr: S, func: F) -> JoinHandle<()>
        where F: FnOnce(io::BufReader<TcpStream>, TcpStream) + Send + 'static
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
        let (mut server, mut handler) = fixture_server();
        let server_addr = handler.server.local_addr().unwrap();
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

        server.run(&mut handler).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn oneshot_reading() {
        let (mut server, mut handler) = fixture_server();
        let server_addr = handler.server.local_addr().unwrap();
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

        server.run(&mut handler).unwrap();
        handle.join().unwrap();
    }
}
