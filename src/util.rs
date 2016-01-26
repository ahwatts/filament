use std::net::SocketAddr;
use rustc_serialize::{Decodable, Decoder};

// Need to wrap SocketAddr with our own type so that we can implement
// RustcDecodable for it.
#[derive(Debug)]
pub struct WrapSocketAddr(pub SocketAddr);

impl Decodable for WrapSocketAddr {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        use std::str::FromStr;
        let addr_str = try!(d.read_str());
        SocketAddr::from_str(&addr_str)
            .map(|a| WrapSocketAddr(a))
            .map_err(|e| d.error(format!("Error parsing address {:?}: {:?}",
                                         addr_str, e).as_ref()))
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
            let addr = try!(SocketAddr::from_str(addr_str).map_err(|e| d.error(&format!("Unable to parse address {:?}: {:?}", addr_str, e))));
            addrs.push(addr);
        }

        Ok(SocketAddrList(addrs))
    }
}
