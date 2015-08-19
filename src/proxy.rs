use super::backend::{TrackerBackend, TrackerMetadata};
use super::error::MogResult;
use std::net::TcpStream;
use url::Url;

#[derive(Debug)]
pub struct ProxyTrackerBackend {
    trackers: Vec<Url>,
    connection: Option<TcpStream>,
}

impl ProxyTrackerBackend {
    pub fn new(trackers: &[Url]) -> ProxyTrackerBackend {
        ProxyTrackerBackend {
            trackers: trackers.to_owned(),
            connection: None,
        }
    }
}

impl TrackerBackend for ProxyTrackerBackend {
    fn create_domain(&self, _domain: &str) -> MogResult<()> {
        unimplemented!()
    }

    fn create_open(&self, _domain: &str, _key: &str) -> MogResult<Vec<Url>> {
        unimplemented!()
    }
    fn create_close(&self, _domain: &str, _key: &str, _path: &Url, _size: u64) -> MogResult<()> {
        unimplemented!()
    }

    fn get_paths(&self, _domain: &str, _key: &str) -> MogResult<Vec<Url>> {
        unimplemented!()
    }

    fn file_info(&self, _domain: &str, _key: &str) -> MogResult<TrackerMetadata> {
        unimplemented!()
    }

    fn delete(&self, _domain: &str, _key: &str) -> MogResult<()> {
        unimplemented!()
    }

    fn rename(&self, _domain: &str, _from: &str, _to: &str) -> MogResult<()> {
        unimplemented!()
    }

    fn list_keys(&self, _domain: &str, _prefix: Option<&str>, _after_key: Option<&str>, _limit: Option<usize>) -> MogResult<Vec<String>> {
        unimplemented!()
    }
}
