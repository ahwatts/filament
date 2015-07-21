use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::error::{MogError, MogResult};
use super::storage::Storage;
use url::Url;

pub use self::model::{Domain, FileInfo};

pub mod model;

#[derive(Debug, Default)]
pub struct Backend {
    domains: HashMap<String, Domain>,
    // files: HashMap<String, FileInfo>,
}

impl Backend {
    pub fn new() -> Backend {
        Backend {
            domains: HashMap::new(),
            // files: HashMap::new()
        }
    }

    pub fn file(&self, domain: &str, key: &str) -> MogResult<Option<&FileInfo>> {
        self.domains.get(domain)
            .ok_or(MogError::UnknownDomain(Some(domain.to_string())))
            .map(|d| d.file(key))
    }

    pub fn file_mut(&mut self, domain: &str, key: &str) -> MogResult<Option<&mut FileInfo>> {
        self.domains.get_mut(domain)
            .ok_or(MogError::UnknownDomain(Some(domain.to_string())))
            .map(|d| d.file_mut(key))
    }


    pub fn create_domain(&mut self, domain_name: &str) -> MogResult<()> {
        if self.domains.contains_key(domain_name) {
            Err(MogError::DuplicateDomain(Some(domain_name.to_string())))
        } else {
            let domain = Domain::new(domain_name);
            self.domains.insert(domain_name.to_string(), domain);
            Ok(())
        }
    }

    pub fn create_open(&mut self, domain_name: &str, key: &str, storage: &Storage) -> MogResult<Vec<Url>> {
        let domain = try!(self.domains.get_mut(domain_name).ok_or(MogError::UnknownDomain(Some(domain_name.to_string()))));
        let file_info = FileInfo::new(key);
        try!(domain.add_file(key, file_info));
        Ok(vec![ storage.url_for_key(domain_name, key) ])
    }

    pub fn create_close(&mut self, _domain: &str, _key: &str, _path: &Url, _size: u64) -> MogResult<()> {
        // There's really nothing to do here; we presumably could
        // verify that the file was uploaded to the URL, but ehh.
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SyncBackend(Arc<Mutex<Backend>>);

impl SyncBackend {
    pub fn new(backend: Backend) -> SyncBackend {
        SyncBackend(Arc::new(Mutex::new(backend)))
    }

    pub fn with_file<F>(&self, domain: &str, key: &str, block: F) -> MogResult<()>
        where F: FnOnce(&FileInfo) -> MogResult<()>
    {
        let guard = try!(self.0.lock());
        match guard.file(domain, key) {
            Ok(Some(ref file_info)) => block(file_info),
            Ok(None) => Err(MogError::UnknownKey(Some(key.to_string()))),
            Err(e) => Err(e),
        }
    }

    pub fn with_file_mut<F>(&self, domain: &str, key: &str, block: F) -> MogResult<()>
        where F: FnOnce(&mut FileInfo) -> MogResult<()>
    {
        let mut guard = try!(self.0.lock());
        match guard.file_mut(domain, key) {
            Ok(Some(ref mut file_info)) => block(file_info),
            Ok(None) => Err(MogError::UnknownKey(Some(key.to_string()))),
            Err(e) => Err(e),
        }
    }

    pub fn create_domain(&self, domain: &str) -> MogResult<()> {
        try!(self.0.lock()).create_domain(domain)
    }

    pub fn create_open(&self, domain: &str, key: &str, storage: &Storage) -> MogResult<Vec<Url>> {
        try!(self.0.lock()).create_open(domain, key, storage)
    }

    pub fn create_close(&self, _domain: &str, _key: &str, _url: &Url, _size: u64) -> MogResult<()> {
        // There's nothing to do here. See the equivalent method on
        // the actual backend. There's no need acquire the mutex and
        // call it, since we're not going to be doing anything with
        // it anyway.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;

    #[test]
    fn test_domain_remove_file() {
        let mut domain = domain_fixture();

        {   // Remove test key 2.
            let remove_result = domain.remove_file(TEST_KEY_2);
            assert!(remove_result.is_some());
            let removed = remove_result.unwrap();
            assert_eq!(TEST_KEY_2, removed.key());
        }

        {   // Make sure it's still not there.
            let get_result = domain.file(TEST_KEY_2);
            assert!(get_result.is_none());
        }

        {   // And you can't remove it again.
            let remove_result_2 = domain.remove_file(TEST_KEY_2);
            assert!(remove_result_2.is_none());
        }
    }

    #[test]
    fn backend_get_file() {
        let mut backend = backend_fixture();

        {   // immutable, file present
            match backend.file(TEST_DOMAIN, TEST_KEY_1) {
                Ok(Some(file)) => {
                    assert_eq!(TEST_KEY_1, file.key());
                },
                v @ _ => panic!("Bad return for getting present file immutably: {:?}", v),
            }
        }

        {   // immutable, file not present
            match backend.file(TEST_DOMAIN, "test/key/3") {
                Ok(None) => {},
                v @ _ => panic!("Bad return for getting missing file immutably: {:?}", v),
            }
        }

        {   // mutable, file present
            match backend.file_mut(TEST_DOMAIN, TEST_KEY_1) {
                Ok(Some(file)) => {
                    assert_eq!(TEST_KEY_1, file.key());
                },
                v @ _ => panic!("Bad return for getting present file mutably: {:?}", v),
            }
        }

        {   // mutable, file not present
            match backend.file_mut(TEST_DOMAIN, "test/key/3") {
                Ok(None) => {},
                v @ _ => panic!("Bad return for getting missing file mutably: {:?}", v),
            }
        }
    }
}

#[cfg(test)]
pub mod test_support {
    use std::collections::HashMap;
    use super::*;

    pub use super::model::test_support::*;

    pub fn backend_fixture() -> Backend {
        let mut backend = Backend {
            domains: HashMap::new(),
        };
        let domain = domain_fixture();
        backend.domains.insert(domain.name().to_string(), domain);
        backend
    }

    pub fn sync_backend_fixture() -> SyncBackend {
        SyncBackend::new(backend_fixture())
    }
}
