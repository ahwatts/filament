extern crate iron;
extern crate libc;
extern crate url;

#[cfg(feature = "evented")] extern crate mio;
#[cfg(feature = "evented")] extern crate threadpool;
#[cfg(test)] extern crate regex;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

pub mod common;
pub mod tracker;
pub mod storage;

#[cfg(feature = "evented")] pub mod ctrlc;

#[cfg(test)]
mod test_support {
    use std::collections::HashMap;
    use super::common::{Backend, SyncBackend, FileInfo};
    use std::sync::{Arc, Mutex};

    pub static TEST_HOST: &'static str = "test.host";
    pub static TEST_BASE_PATH: &'static str = "base_path";

    pub static TEST_KEY_1: &'static str = "test/key/1";
    pub static TEST_CONTENT_1: &'static [u8] = b"This is test content";

    pub static TEST_KEY_2: &'static str = "test/key/2";

    pub fn backend_fixture() -> Backend {
        let mut backend_hash = Backend(HashMap::new());

        backend_hash.0.insert(
            TEST_KEY_1.to_string(),
            FileInfo {
                key: TEST_KEY_1.to_string(),
                content: Some(Vec::from(TEST_CONTENT_1)),
                size: Some(TEST_CONTENT_1.len()),
            });

        backend_hash.0.insert(
            TEST_KEY_2.to_string(),
            FileInfo {
                key: TEST_KEY_2.to_string(),
                content: None,
                size: None,
            });

        backend_hash
    }

    pub fn sync_backend_fixture() -> SyncBackend {
        Arc::new(Mutex::new(backend_fixture()))
    }
}
