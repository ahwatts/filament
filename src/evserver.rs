use mio::buf::{Buf, MutBuf, RingBuf};
use mio::tcp::{self, TcpListener, TcpStream};
use mio::{EventLoop, Handler, Interest, NonBlock, PollOpt, ReadHint, Token, TryRead, TryWrite};
use std::collections::HashMap;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Cursor};
use std::net::ToSocketAddrs;
use std::result;
use std::rc::Rc;
use super::tracker::Tracker;

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

        Ok(try!(self.event_loop.run(handler)))
    }
}

pub struct ServerHandler {
    server: NonBlock<TcpListener>,
    token: Token,
    conns: HashMap<Token, Connection>,
    last_token: Token,
    tracker: Rc<Tracker>,
}

impl ServerHandler {
    pub fn new<T: ToSocketAddrs>(sock_addr: T) -> Result<ServerHandler> {
        let sock_addr = try!(try!(sock_addr.to_socket_addrs()).next().ok_or(Error::Other));
        let socket = try!(tcp::listen(&sock_addr));
        let token = Token(0);

        let handler = ServerHandler {
            server: socket,
            token: token,
            conns: HashMap::new(),
            last_token: token,
            tracker: Rc::new(Tracker::new()),
        };

        Ok(handler)
    }

    fn accept(&mut self, event_loop: &mut EventLoop<Self>) -> Result<()> {
        let stream = try!(try!(self.server.accept()).ok_or(Error::Other));
        let conn = Connection::new(stream, Token(self.last_token.as_usize() + 1), self.tracker.clone());
        try!(event_loop.register_opt(&conn.sock, conn.token, conn.interest, PollOpt::edge()));
        println!("New connection {:?} from {:?}", conn.token, conn.sock.peer_addr());
        self.last_token = conn.token;
        self.conns.insert(conn.token, conn);
        Ok(())
    }
}

impl Handler for ServerHandler {
    type Timeout = usize;
    type Message = ();

    fn readable(&mut self, event_loop: &mut EventLoop<Self>, token: Token, _: ReadHint) {
        match token {
            t if t == self.token => {
                self.accept(event_loop).unwrap_or_else(|e| {
                    println!("Error accepting connection: {}", e);
                });
            },
            t if self.conns.contains_key(&t) => {
                let conn = self.conns.get_mut(&t).unwrap();
                conn.readable(event_loop).unwrap_or_else(|e| {
                    println!("Error handling readable event for connection {:?}: {}", t, e);
                });
            },
            _ => {
                println!("Readable event for unknown connection {:?}", token);
            }
        }
    }

    fn writable(&mut self, event_loop: &mut EventLoop<Self>, token: Token) {
        match token {
            t if self.conns.contains_key(&t) => {
                let conn = self.conns.get_mut(&t).unwrap();
                conn.writable(event_loop).unwrap_or_else(|e| {
                    println!("Error handling writable event for connection {:?}: {}", t, e);
                });
            }
            _ => {
                println!("Writable event for unknown connection: {:?}", token);
            }
        }
    }

    fn notify(&mut self, _: &mut EventLoop<Self>, message: ()) {
        println!("Notify event: message = {:?}", message);
    }

    fn timeout(&mut self, _: &mut EventLoop<Self>, timeout: usize) {
        println!("Timeout event: timeout = {:?}", timeout);
    }

    fn interrupted(&mut self, _: &mut EventLoop<Self>) {
        println!("Interruted event.");
    }
}

pub struct Connection {
    sock: NonBlock<TcpStream>,
    token: Token,
    in_buf: RingBuf,
    out_buf: RingBuf,
    interest: Interest,
    tracker: Rc<Tracker>,
}

impl Connection {
    pub fn new(sock: NonBlock<TcpStream>, token: Token, tracker: Rc<Tracker>) -> Connection {
        Connection {
            sock: sock,
            token: token,
            in_buf: RingBuf::new(2048),
            out_buf: RingBuf::new(2048),
            interest: Interest::readable(),
            tracker: tracker,
        }
    }

    fn readable<S: Handler>(&mut self, event_loop: &mut EventLoop<S>) -> Result<()> {
        let bytes_read = try!(try!(self.sock.read(&mut self.in_buf)).ok_or(Error::Other));
        println!("Read {} bytes from {:?}", bytes_read, self.token);

        match self.extract_line(&self.in_buf) {
            Some(line) => {
                Buf::advance(&mut self.in_buf, line.len() + 2);
                // self.in_buf.advance(line.len() + 2);
                let response = self.tracker.handle(&mut Cursor::new(line.as_ref()));
                self.out_buf.write_slice(response.render().as_bytes());
                self.interest = Interest::writable();
                try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
            },
            None => {}
        }

        Ok(())
    }

    fn writable<S: Handler>(&mut self, event_loop: &mut EventLoop<S>) -> Result<()> {
        let has_line = self.has_line(&self.out_buf);
        let bytes_wrote = try!(try!(self.sock.write(&mut self.out_buf)).ok_or(Error::Other));
        println!("Wrote {} bytes to {:?}", bytes_wrote, self.token);

        if has_line {
            self.interest = Interest::readable();
            try!(event_loop.reregister(&self.sock, self.token, self.interest, PollOpt::edge()));
        }

        Ok(())
    }

    fn has_line<T: Buf>(&self, buf: &T) -> bool {
        for w in buf.bytes().windows(2) {
            if w == &[ '\r' as u8, '\n' as u8 ] {
                return true;
            }
        }
        false
    }

    fn extract_line<T: Buf>(&self, buf: &T) -> Option<Vec<u8>> {
        let bytes = buf.bytes();
        let mut line = None;

        for (i, w) in bytes.windows(2).enumerate() {
            if w == &[ '\r' as u8, '\n' as u8 ] {
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
    Other,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(ref io_err) => io_err.description(),
            Error::Other => "Other error",
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Error::IoError(ref io_err) => write!(f, "{}", io_err),
            Error::Other => write!(f, "Other error"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(io_err: io::Error) -> Error {
        Error::IoError(io_err)
    }
}

pub type Result<T> = result::Result<T, Error>;
