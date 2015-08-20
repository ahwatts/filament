extern crate rand;
#[macro_use] extern crate log;

use std::io::{self, Read, Write, BufRead, BufReader, BufWriter};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

#[derive(Debug)]
pub struct MogClient {
    trackers: Vec<SocketAddr>,
    transport: Option<MogClientTransport>,
}

impl MogClient {
    pub fn new<S: ToSocketAddrs + Sized>(trackers: &[S]) -> MogClient {
        let sock_addrs = trackers.iter().flat_map(|a| a.to_socket_addrs().unwrap()).collect();
        debug!("sock_addrs = {:?}", sock_addrs);
        MogClient {
            trackers: sock_addrs,
            transport: None,
        }
    }

    pub fn file_info(&mut self, domain: &str, key: &str) -> MogClientResult<Response> {
        try!(self.ensure_connected());
        self.transport.as_mut()
            .ok_or(MogClientError::NoConnection)
            .and_then(|mut t| t.do_request(&Request::file_info(domain, key)))
    }

    fn random_tracker_addr(&self) -> MogClientResult<SocketAddr> {
        let mut rng = rand::thread_rng();
        let mut sample = rand::sample(&mut rng, self.trackers.iter(), 1);
        sample.pop().cloned().ok_or(MogClientError::NoTrackers)
    }

    fn ensure_connected(&mut self) -> MogClientResult<()> {
        if self.transport.is_some() {
            Ok(())
        } else {
            let tracker = try!(self.random_tracker_addr());
            let conn = try!(MogClientTransport::connect(&tracker));
            self.transport = Some(conn);
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct MogClientTransport {
    read: BufReader<TcpStream>,
    write: BufWriter<TcpStream>,
}

impl MogClientTransport {
    pub fn connect<S: ToSocketAddrs + ?Sized>(tracker_addr: &S) -> MogClientResult<MogClientTransport> {
        let stream = try!(TcpStream::connect(tracker_addr));
        debug!("stream = {:?}", stream);

        Ok(MogClientTransport {
            read: BufReader::new(try!(stream.try_clone())),
            write: BufWriter::new(stream),
        })
    }

    pub fn do_request(&mut self, request: &Request) -> MogClientResult<Response> {
        let mut line = String::new();
        try!(self.write.write_all(format!("{}\r\n", request.line()).as_bytes()));
        try!(self.write.flush());
        try!(self.read.read_line(&mut line));
        Response::from_line(&line)
    }
}

#[derive(Debug)]
pub enum Request {
    FileInfo { domain: String, key: String },
}

impl Request {
    pub fn line(&self) -> String {
        use self::Request::*;

        match self {
            &FileInfo { ref domain, ref key } => {
                format!("file_info domain={}&key={}", domain, key)
            }
        }
    }

    pub fn file_info(domain: &str, key: &str) -> Request {
        Request::FileInfo { domain: domain.to_string(), key: key.to_string() }
    }
}

#[derive(Debug)]
pub enum Response {}

impl Response {
    pub fn from_line(line: &str) -> MogClientResult<Response> {
        info!("Response from MogileFS: {:?}", line);
        unimplemented!();
    }
}

pub type MogClientResult<T> = Result<T, MogClientError>;

#[derive(Debug)]
pub enum MogClientError {
    IoError(io::Error),
    NoConnection,
    NoTrackers,
}

impl From<io::Error> for MogClientError {
    fn from(ioe: io::Error) -> MogClientError {
        MogClientError::IoError(ioe)
    }
}
