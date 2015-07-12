use mio::buf::{ByteBuf, MutByteBuf};
use mio::tcp::{TcpListener, TcpStream};
use mio::{EventLoop, Handler, Interest, NonBlock, PollOpt, ReadHint, Token};
use std::collections::HashMap;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::result;

pub struct Server {
    server: NonBlock<TcpListener>,
    token: Token,
    conns: HashMap<Token, Connection>,
    last_token: Token,
}

impl Server {
    pub fn new(socket: NonBlock<TcpListener>, token: Token) -> Server {
        Server {
            server: socket,
            token: token,
            conns: HashMap::new(),
            last_token: token,
        }
    }

    fn accept(&mut self, event_loop: &mut EventLoop<Self>) -> Result<()> {
        let stream = try!(try!(self.server.accept()).ok_or(Error::Other));
        let conn = Connection::new(stream, Token(self.last_token.as_usize() + 1));
        try!(event_loop.register_opt(&conn.sock, conn.token, conn.interest, PollOpt::edge()));
        self.last_token = conn.token;
        self.conns.insert(conn.token, conn);
        Ok(())
    }
}

impl Handler for Server {
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
                conn.readable().unwrap_or_else(|e| {
                    println!("Error handling readable event for connection {:?}: {}", t, e);
                });
            },
            _ => {
                println!("Readable event for unknown connection {:?}", token);
            }
        }
    }
}

pub struct Connection {
    sock: NonBlock<TcpStream>,
    token: Token,
    buf: Option<ByteBuf>,
    mut_buf: Option<MutByteBuf>,
    interest: Interest,
}

impl Connection {
    pub fn new(sock: NonBlock<TcpStream>, token: Token) -> Connection {
        Connection {
            sock: sock,
            token: token,
            buf: None,
            mut_buf: Some(ByteBuf::mut_with_capacity(2048)),
            interest: Interest::readable(),
        }
    }

    fn readable(&mut self) -> Result<()> {
        Ok(())
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
