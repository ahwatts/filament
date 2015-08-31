extern crate bufstream;
extern crate mogilefs_common;
extern crate rand;
extern crate url;

#[macro_use] extern crate log;

use bufstream::BufStream;
use mogilefs_common::requests::*;
use mogilefs_common::{Request, Response, MogError, MogResult, BufReadMb, FromBytes};
use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use to_args::ToArgs;
use url::form_urlencoded;

mod to_args;

trait ClientRequest: Request + ToArgs {
    fn render(&self) -> String {
        format!("{} {}", self.op(), form_urlencoded::serialize(self.args()))
    }
}

impl<R: Request + ToArgs> ClientRequest for R {}

#[derive(Debug)]
pub struct MogClient {
    transport: MogClientTransport,
}

impl MogClient {
    pub fn new<S: ToSocketAddrs + Sized>(trackers: &[S]) -> MogClient {
        MogClient {
            transport: MogClientTransport::new(trackers),
        }
    }

    pub fn file_info(&mut self, domain: &str, key: &str) -> MogResult<Response> {
        let req = FileInfo {
            domain: domain.to_string(),
            key: key.to_string(),
        };
        info!("request = {:?}", req);
        let resp_rslt = self.transport.do_request(req);
        info!("response = {:?}", resp_rslt);
        resp_rslt
    }
}

#[derive(Debug)]
struct MogClientTransport {
    hosts: Vec<SocketAddr>,
    stream: Option<BufStream<TcpStream>>,
}

impl MogClientTransport {
    pub fn new<S: ToSocketAddrs + Sized>(tracker_addrs: &[S]) -> MogClientTransport {
        MogClientTransport {
            hosts: tracker_addrs.iter().flat_map(|a| a.to_socket_addrs().unwrap()).collect(),
            stream: None,
        }
    }

    fn random_tracker_addr(&self) -> MogResult<SocketAddr> {
        let mut rng = rand::thread_rng();
        let mut sample = rand::sample(&mut rng, self.hosts.iter(), 1);
        sample.pop().cloned().ok_or(MogError::NoTrackers)
    }

    fn ensure_connected(&mut self) -> MogResult<()> {
        match self.stream {
            None => {
                let tracker = try!(self.random_tracker_addr());
                let tcp_stream = try!(TcpStream::connect(tracker));
                self.stream = Some(BufStream::new(tcp_stream));
                Ok(())
            },
            _ => Ok(()),
        }
    }

    pub fn do_request<R: ClientRequest>(&mut self, request: R) -> MogResult<Response> {
        try!(self.ensure_connected());

        match self.stream {
            Some(ref mut stream) => {
                let mut resp_line = Vec::new();
                let req_line = format!("{}\r\n", request.render());
                debug!("req_line = {:?}", req_line);
                try!(stream.write_all(req_line.as_bytes()));
                try!(stream.flush());
                try!(stream.read_until_mb(b"\r\n", &mut resp_line));
                debug!("resp_line = {:?}", String::from_utf8_lossy(&resp_line));

                if resp_line.ends_with(b"\r\n") {
                    let len = resp_line.len();
                    resp_line = resp_line.into_iter().take(len - 2).collect();
                }

                Response::from_bytes(&resp_line)
            },
            None => {
                Err(MogError::NoConnection)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info() {
        let mut client = MogClient::new(&[ "127.0.0.1:7001" ]);
        let response = client.file_info("rn_development_private", "Song/225322/image");
        println!("response = {:?}", response);
    }
}
