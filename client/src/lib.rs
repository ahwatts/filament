extern crate mogilefs_common;
extern crate rand;
extern crate url;

#[macro_use] extern crate log;

use mogilefs_common::{MogError, MogResult};
use std::io::{Read, Write, BufRead, BufReader, BufWriter};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
pub use message::{Request, Response};

mod message;

#[derive(Debug)]
pub struct MogClient {
    trackers: Vec<SocketAddr>,
    transport: Option<MogClientTransport>,
}

impl MogClient {
    pub fn new<S: ToSocketAddrs + Sized>(trackers: &[S]) -> MogClient {
        let sock_addrs = trackers.iter().flat_map(|a| a.to_socket_addrs().unwrap()).collect();
        MogClient {
            trackers: sock_addrs,
            transport: None,
        }
    }

    pub fn file_info(&mut self, domain: &str, key: &str) -> MogResult<Response> {
        try!(self.ensure_connected());
        self.transport.as_mut()
            .ok_or(MogError::NoConnection)
            .and_then(|mut t| t.do_request(&Request::file_info(domain, key)))
    }

    fn random_tracker_addr(&self) -> MogResult<SocketAddr> {
        let mut rng = rand::thread_rng();
        let mut sample = rand::sample(&mut rng, self.trackers.iter(), 1);
        sample.pop().cloned().ok_or(MogError::NoTrackers)
    }

    fn ensure_connected(&mut self) -> MogResult<()> {
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
    pub fn connect<S: ToSocketAddrs + ?Sized>(tracker_addr: &S) -> MogResult<MogClientTransport> {
        let stream = try!(TcpStream::connect(tracker_addr));
        debug!("stream = {:?}", stream);

        Ok(MogClientTransport {
            read: BufReader::new(try!(stream.try_clone())),
            write: BufWriter::new(stream),
        })
    }

    pub fn do_request(&mut self, request: &Request) -> MogResult<Response> {
        let mut line = String::new();
        try!(self.write.write_all(format!("{}\r\n", request.line()).as_bytes()));
        try!(self.write.flush());
        try!(self.read.read_line(&mut line));
        Ok(Response::from_line(&line))
    }
}
