use lookup::lookup;
use std::net::SocketAddr;
use std::str::FromStr;
use rustc_serialize::{Decodable, Decoder};

// Need to wrap SocketAddr with our own type so that we can implement
// RustcDecodable for it.
#[derive(Debug)]
pub struct WrapSocketAddr(pub SocketAddr);

impl Decodable for WrapSocketAddr {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use std::str::FromStr;
        let addr_str = try!(d.read_str());
        WrapSocketAddr::from_str(&addr_str)
            .map_err(|e| d.error(e.as_ref()))
    }
}

impl FromStr for WrapSocketAddr {
    type Err = String;

    #[allow(unused_assignments)]
    fn from_str(addr_port_str: &str) -> Result<WrapSocketAddr, String> {
        let mut addr_port = addr_port_str.split(":");
        let addr_str = addr_port.next().unwrap();
        let port_str = addr_port.next().unwrap();
        let ips = try!(lookup(addr_str));
        ips.first()
            .ok_or(format!("No IPs found for {:?}", addr_str))
            .map(|ip| {
                let sa = SocketAddr::new(*ip, FromStr::from_str(port_str).unwrap());
                WrapSocketAddr(sa)
            })
    }
}

#[derive(Debug)]
pub struct SocketAddrList(pub Vec<SocketAddr>);

impl SocketAddrList {
    pub fn as_slice(&self) -> &[SocketAddr] {
        &self.0
    }
}

impl Decodable for SocketAddrList {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use std::str::FromStr;

        let addrs_str = try!(d.read_str());
        let mut addrs = Vec::new();

        for addr_str in addrs_str.split(',') {
            let addr = try!(WrapSocketAddr::from_str(addr_str).map_err(|e| d.error(&format!("Unable to parse address {:?}: {:?}", addr_str, e))));
            addrs.push(addr.0);
        }

        Ok(SocketAddrList(addrs))
    }
}
