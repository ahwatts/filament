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
}

impl Backend {
    pub fn new() -> Backend {
        Backend {
            domains: HashMap::new(),
        }
    }

    pub fn file(&self, domain: &str, key: &str) -> MogResult<Option<&FileInfo>> {
        self.domain(domain).map(|d| d.file(key))
    }

    pub fn file_mut(&mut self, domain: &str, key: &str) -> MogResult<Option<&mut FileInfo>> {
        self.domain_mut(domain).map(|d| d.file_mut(key))
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
        let domain = try!(self.domain_mut(domain_name));
        let file_info = FileInfo::new(key);
        try!(domain.add_file(key, file_info));
        Ok(vec![ storage.url_for_key(domain_name, key) ])
    }

    pub fn create_close(&mut self, _domain: &str, _key: &str, _path: &Url, _size: u64) -> MogResult<()> {
        // There's really nothing to do here; we presumably could
        // verify that the file was uploaded to the URL, but ehh.
        Ok(())
    }

    pub fn list_keys(&mut self, domain_name: &str, after_key: &str, limit: usize) -> MogResult<Vec<String>> {
        Ok(try!(self.domain(domain_name)).files()
            .skip_while(|&(k, _)| k <= after_key)
            .take(limit)
            .map(|(k, _)| k.to_string())
            .collect())
    }

    fn domain(&self, domain_name: &str) -> MogResult<&Domain> {
        self.domains.get(domain_name).ok_or(MogError::UnknownDomain(Some(domain_name.to_string())))
    }

    fn domain_mut(&mut self, domain_name: &str) -> MogResult<&mut Domain> {
        self.domains.get_mut(domain_name).ok_or(MogError::UnknownDomain(Some(domain_name.to_string())))
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

    pub fn list_keys(&self, domain: &str, after_key: &str, limit: usize) -> MogResult<Vec<String>> {
        try!(self.0.lock()).list_keys(domain, after_key, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::super::error::MogError;
    use super::test_support::*;

    #[test]
    fn backend_get_file() {
        let mut backend = backend_fixture();

        {
            let file = backend.file(TEST_DOMAIN, TEST_KEY_1);
            assert!(
                matches!(file, Ok(Some(ref f)) if f.key() == TEST_KEY_1),
                "Immutable present file was {:?}", file);
        }

        {
            let file = backend.file(TEST_DOMAIN, "test/key/3");
            assert!(
                matches!(file, Ok(None)),
                "Immutable missing file was {:?}", file);
        }

        {
            let file = backend.file_mut(TEST_DOMAIN, TEST_KEY_1);
            assert!(
                matches!(file, Ok(Some(ref f)) if f.key() == TEST_KEY_1),
                "Mutable present file was {:?}", file);
        }

        {
            let file = backend.file_mut(TEST_DOMAIN, "test/key/3");
            assert!(
                matches!(file, Ok(None)),
                "Mutable missing file was {:?}", file);
        }

        {
            let file = backend.file("test_domain_2", TEST_KEY_1);
            assert!(
                matches!(file, Err(MogError::UnknownDomain(Some(ref d))) if d == "test_domain_2"),
                "Immutable file from nonexistent domain was {:?}", file);
        }

        {
            let file = backend.file_mut("test_domain_2", TEST_KEY_1);
            assert!(
                matches!(file, Err(MogError::UnknownDomain(Some(ref d))) if d == "test_domain_2"),
                "Mutable file from nonexistent domain was {:?}", file);
        }
    }

    #[test]
    fn test_backend_create_domain() {
        let mut backend = backend_fixture();

        let create_result = backend.create_domain("test_domain_2");
        assert!(create_result.is_ok(), "Create new domain result was {:?}", create_result);

        assert!(backend.domains.contains_key("test_domain_2"));

        let create_dup_result = backend.create_domain(TEST_DOMAIN);
        assert!(
            matches!(create_dup_result, Err(MogError::DuplicateDomain(Some(ref d))) if d == TEST_DOMAIN),
            "Create duplicate domain result was {:?}", create_dup_result);
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
