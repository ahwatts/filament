extern crate bufstream;
extern crate mogilefs_common;
extern crate rand;
extern crate url;

#[macro_use] extern crate log;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use bufstream::BufStream;
use mogilefs_common::{Request, Response, MogError, MogResult, BufReadMb, ToArgs};
use mogilefs_common::requests::*;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use url::{form_urlencoded, percent_encoding};

#[derive(Debug)]
pub struct MogClient {
    transport: MogClientTransport,
}

impl MogClient {
    pub fn new<S: ToSocketAddrs>(trackers: &[S]) -> MogClient {
        MogClient {
            transport: MogClientTransport::new(trackers),
        }
    }

    pub fn request<R: Request + ToArgs + ?Sized>(&mut self, req: &R) -> MogResult<Response> {
        info!("request = {:?}", req);
        let resp_rslt = self.transport.do_request(req);
        info!("response = {:?}", resp_rslt);
        resp_rslt
    }

    pub fn store_data<R: Read>(&mut self, domain: String, class: Option<String>, key: String, _data: &mut R) -> MogResult<Response> {
        let open_req = CreateOpen { domain: domain.clone(), class: class, key: key.clone(), multi_dest: true, size: None };
        let open_res = self.request(&open_req);

        println!("open_res = {:?}", open_res);

        unimplemented!()
    }

    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.transport.stream.as_ref().and_then(|s| s.peer_addr())
    }
}

#[derive(Debug)]
struct MogClientTransport {
    hosts: Vec<SocketAddr>,
    stream: Option<ConnectionState>,
}

impl MogClientTransport {
    fn new<S: ToSocketAddrs + Sized>(tracker_addrs: &[S]) -> MogClientTransport {
        MogClientTransport {
            hosts: tracker_addrs.iter().flat_map(|a| a.to_socket_addrs().unwrap()).collect(),
            stream: Some(ConnectionState::new()),
        }
    }

    fn is_connected(&self) -> bool {
        match self.stream.as_ref() {
            Some(ref stream) => stream.is_connected(),
            None => false,
        }
    }

    fn random_tracker_addr(&self) -> MogResult<SocketAddr> {
        let mut rng = rand::thread_rng();
        let mut sample = rand::sample(&mut rng, self.hosts.iter(), 1);
        sample.pop().cloned().ok_or(MogError::NoTrackers)
    }

    fn do_request<R: Request + ?Sized>(&mut self, request: &R) -> MogResult<Response> {
        let mut stream = self.stream.take().unwrap_or(ConnectionState::new());
        let req_line = format!("{} {}\r\n", request.op(), form_urlencoded::serialize(request.to_args()));
        let mut resp_line = Vec::new();
        let mut tries = 0;

        loop {
            if !stream.is_connected() {
                let tracker = try!(self.random_tracker_addr());
                debug!("Connecting to {:?}", tracker);
                stream = stream.connect(&tracker);
            }

            debug!("req_line = {:?}", req_line);
            stream = stream.write_and_flush(req_line.as_bytes());
            stream = stream.read_until_mb(&mut resp_line);
            debug!("resp_line = {:?}", String::from_utf8_lossy(&resp_line));
            tries += 1;

            if stream.is_connected() || tries >= 3 { break; }
        }


        let (stream, err) = stream.take_err();
        self.stream = Some(stream);

        match err {
            Some(err) => Err(MogError::Io(err)),
            None => {
                if resp_line.ends_with(b"\r\n") {
                    let len = resp_line.len();
                    resp_line = resp_line.into_iter().take(len - 2).collect();
                }
                response_from_bytes(request, &resp_line)
            }
        }
    }
}

fn response_from_bytes<R: Request + ?Sized>(request: &R, bytes: &[u8]) -> MogResult<Response> {
    let mut toks = bytes.splitn(2, |&b| b == b' ');
    let op = toks.next();
    let args = toks.next().unwrap_or(&[]);

    match op {
        Some(b"OK") => request.response_from_bytes(&args),
        Some(b"ERR") => Err(MogError::from_bytes(&args)),
        o @ _ => {
            let err_str = o.map(|bs| {
                percent_encoding::lossy_utf8_percent_decode(bs)
                    .replace("+", " ")
            });
            Err(MogError::Other("Unknown response code".to_string(), err_str))
        },
    }
}

#[derive(Debug)]
enum ConnectionState {
    NoConnection,
    Connected(BufStream<TcpStream>),
    Error(io::Error),
}

impl ConnectionState {
    fn new() -> ConnectionState {
        ConnectionState::NoConnection
    }

    fn is_connected(&self) -> bool {
        match self {
            &ConnectionState::Connected(..) => true,
            _ => false,
        }
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        match self {
            &ConnectionState::Connected(ref stream) => {
                stream.get_ref().peer_addr().ok()
            },
            _ => None,
        }
    }

    fn take_err(self) -> (ConnectionState, Option<io::Error>) {
        use self::ConnectionState::*;

        match self {
            ConnectionState::Error(ioe) => (NoConnection, Some(ioe)),
            _ => (self, None),
        }
    }

    fn connect(self, addr: &SocketAddr) -> ConnectionState {
        use self::ConnectionState::*;

        match self {
            Connected(..) => self,
            _ => {
                trace!("Opening connection to {:?}...", addr);
                match TcpStream::connect(addr) {
                    Ok(stream) => {
                        trace!("... connected to {:?}", addr);
                        Connected(BufStream::new(stream))
                    },
                    Err(ioe) => {
                        error!("Error connecting to {:?}: {}", addr, ioe);
                        Error(ioe)
                    },
                }
            },
        }
    }

    fn write_and_flush(self, line: &[u8]) -> ConnectionState {
        use self::ConnectionState::*;

        match self {
            NoConnection | Error(..) => self,
            Connected(mut stream) => {
                let peer = stream.get_ref().peer_addr();
                trace!("Writing {} bytes to {:?}...", line.len(), peer);
                match stream.write_all(line).and_then(|_| stream.flush()) {
                    Ok(..) => {
                        trace!("... successfully wrote {} bytes to {:?}", line.len(), peer);
                        Connected(stream)
                    },
                    Err(ioe) => {
                        error!("Error writing to {:?}: {}", peer, ioe);
                        Error(ioe)
                    }
                }
            },
        }
    }

    fn read_until_mb(self, buf: &mut Vec<u8>) -> ConnectionState {
        use self::ConnectionState::*;

        match self {
            NoConnection | Error(..) => self,
            Connected(mut stream) => {
                let peer = stream.get_ref().peer_addr();
                trace!("Waiting for response from {:?}...", peer);
                match stream.read_until_mb(b"\r\n", buf) {
                    Ok(..) => {
                        trace!("... read {} bytes from {:?}", buf.len(), peer);
                        Connected(stream)
                    },
                    Err(ioe) => {
                        error!("Error reading from {:?}: {}", peer, ioe);
                        Error(ioe)
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mogilefs_common::Response;
    use mogilefs_common::requests::*;
    use std::env;
    use std::io::{self, Cursor, Write};
    use std::net::SocketAddr;
    use std::str::FromStr;
    use super::*;

    lazy_static!{
        static ref TEST_DOMAIN: String = domain_for_testing();
    }

    fn trackers_for_testing() -> Option<Vec<SocketAddr>> {
        env::var("FILAMENT_TEST_TRACKERS").map(|val| {
            val.split(",")
                .into_iter()
                .filter_map(|addr_str| SocketAddr::from_str(addr_str).ok())
                .collect()
        }).ok()
    }

    fn domain_for_testing() -> String {
        env::var("FILAMENT_TEST_DOMAIN").ok().unwrap_or("filament_test".to_string())
    }

    fn skip() {
        write!(&mut io::stdout(), "(skipped) ").unwrap();
    }

    macro_rules! test_conn {
        () => {
            {
                let trackers = match trackers_for_testing() {
                    Some(vec) => vec,
                    None => {
                        skip();
                        return;
                    },
                };
                assert!(trackers.len() >= 1);
                MogClient::new(&trackers)
            }
        }
    }

    #[test]
    fn test_connection() {
        let mut conn = test_conn!();
        let response = conn.request(&Noop);
        assert!(response.is_ok());
        assert_eq!(Response::Empty, response.ok().unwrap());
        assert!(conn.is_connected());
    }

    #[test]
    fn test_store_data() {
        let mut conn = test_conn!();
        let content: Vec<u8> = b"New file content".iter().cloned().collect();
        let mut content_reader = Cursor::new(content);
        let response = conn.store_data(TEST_DOMAIN.clone(), None, "test/key/1".to_string(), &mut content_reader);
        assert!(response.is_ok());
    }
}
