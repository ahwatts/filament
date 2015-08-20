extern crate mogilefsd;
extern crate env_logger;

#[macro_use] extern crate log;

use mogilefsd::client::MogClient;

fn main() {
    env_logger::init().unwrap();
    let mut client = MogClient::new(&[ "127.0.0.1:7001" ]);
    debug!("client = {:?}", client);
    let fi = client.file_info("rn_development_private", "Song/225322/image");
    println!("fi = {:?}", fi);
}
