use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::super::error::{MogError, MogResult};
use super::super::storage::Storage;
use super::{Domain, FileInfo};
use url::Url;

#[derive(Debug, Default)]
pub struct MemBackend {
    // storage: MemStorage,
    domains: HashMap<String, Domain>,
    empty_domain: Domain,
}

impl MemBackend {
    pub fn new() -> MemBackend {
        MemBackend {
            domains: HashMap::new(),
            empty_domain: Domain::new(""),
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
            Err(MogError::DomainExists(domain_name.to_string()))
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

    pub fn get_paths(&self, domain: &str, key: &str, storage: &Storage) -> MogResult<Vec<Url>> {
        self.domain(domain)
            .and_then(|d| d.file(key).ok_or(MogError::UnknownKey(key.to_string())))
            .map(|_| vec![ storage.url_for_key(domain, key) ])
    }

    pub fn delete(&mut self, domain: &str, key: &str) -> MogResult<()> {
        try!(self.domain_mut(domain))
            .remove_file(key)
            .map(|_| ())
            .ok_or(MogError::UnknownKey(key.to_string()))
    }

    pub fn rename(&mut self, domain: &str, from: &str, to: &str) -> MogResult<()> {
        self.domain_mut(domain).and_then(|d| d.rename(from, to))
    }

    pub fn list_keys(&self, domain_name: &str, prefix: Option<&str>, after_key: Option<&str>, limit: Option<usize>) -> MogResult<Vec<String>> {
        let after_key = after_key.unwrap_or("");
        let prefix = prefix.unwrap_or("");
        let limit = limit.unwrap_or(1000);
        Ok(try!(self.domain(domain_name)).files()
            .skip_while(|&(k, _)| k <= after_key || !k.starts_with(prefix))
            .take(limit)
            .map(|(k, _)| k.to_string())
            .collect())
    }

    fn domain(&self, domain_name: &str) -> MogResult<&Domain> {
        // self.domains.get(domain_name).ok_or(MogError::UnregDomain(domain_name.to_string()))
        Ok(self.domains.get(domain_name).unwrap_or(&self.empty_domain))
    }

    fn domain_mut(&mut self, domain_name: &str) -> MogResult<&mut Domain> {
        // self.domains.get_mut(domain_name).ok_or(MogError::UnregDomain(domain_name.to_string()))
        Ok(self.domains.entry(domain_name.to_string()).or_insert(Domain::new(domain_name)))
    }
}

#[derive(Clone, Debug)]
pub struct SyncMemBackend(Arc<RwLock<MemBackend>>);

impl SyncMemBackend {
    pub fn new(backend: MemBackend) -> SyncMemBackend {
        SyncMemBackend(Arc::new(RwLock::new(backend)))
    }

    pub fn with_file<F>(&self, domain: &str, key: &str, block: F) -> MogResult<()>
        where F: FnOnce(&FileInfo) -> MogResult<()>
    {
        let guard = try!(self.0.read());
        match guard.file(domain, key) {
            Ok(Some(ref file_info)) => block(file_info),
            Ok(None) => Err(MogError::UnknownKey(key.to_string())),
            Err(e) => Err(e),
        }
    }

    pub fn with_file_mut<F>(&self, domain: &str, key: &str, block: F) -> MogResult<()>
        where F: FnOnce(&mut FileInfo) -> MogResult<()>
    {
        let mut guard = try!(self.0.write());
        match guard.file_mut(domain, key) {
            Ok(Some(ref mut file_info)) => block(file_info),
            Ok(None) => Err(MogError::UnknownKey(key.to_string())),
            Err(e) => Err(e),
        }
    }

    pub fn create_domain(&self, domain: &str) -> MogResult<()> {
        try!(self.0.write()).create_domain(domain)
    }

    pub fn create_open(&self, domain: &str, key: &str, storage: &Storage) -> MogResult<Vec<Url>> {
        try!(self.0.write()).create_open(domain, key, storage)
    }

    pub fn create_close(&self, _domain: &str, _key: &str, _url: &Url, _size: u64) -> MogResult<()> {
        // There's nothing to do here. See the equivalent method on
        // the actual backend. There's no need acquire the mutex and
        // call it, since we're not going to be doing anything with
        // it anyway.
        Ok(())
    }

    pub fn get_paths(&self, domain: &str, key: &str, storage: &Storage) -> MogResult<Vec<Url>> {
        try!(self.0.read()).get_paths(domain, key, storage)
    }

    pub fn delete(&self, domain: &str, key: &str) -> MogResult<()> {
        try!(self.0.write()).delete(domain, key)
    }

    pub fn rename(&self, domain: &str, from: &str, to: &str) -> MogResult<()> {
        try!(self.0.write()).rename(domain, from, to)
    }

    pub fn list_keys(&self, domain: &str, prefix: Option<&str>, after_key: Option<&str>, limit: Option<usize>) -> MogResult<Vec<String>> {
        try!(self.0.read()).list_keys(domain, prefix, after_key, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::error::MogError;
    use super::super::super::test_support::*;

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

        // {
        //     let file = backend.file("test_domain_2", TEST_KEY_1);
        //     assert!(
        //         matches!(file, Err(MogError::UnregDomain(ref d)) if d == "test_domain_2"),
        //         "Immutable file from nonexistent domain was {:?}", file);
        // }

        // {
        //     let file = backend.file_mut("test_domain_2", TEST_KEY_1);
        //     assert!(
        //         matches!(file, Err(MogError::UnregDomain(ref d)) if d == "test_domain_2"),
        //         "Mutable file from nonexistent domain was {:?}", file);
        // }
    }

    #[test]
    fn backend_create_domain() {
        let mut backend = backend_fixture();

        let create_result = backend.create_domain("test_domain_2");
        assert!(create_result.is_ok(), "Create new domain result was {:?}", create_result);

        assert!(backend.domains.contains_key("test_domain_2"));

        let create_dup_result = backend.create_domain(TEST_DOMAIN);
        assert!(
            matches!(create_dup_result, Err(MogError::DomainExists(ref d)) if d == TEST_DOMAIN),
            "Create duplicate domain result was {:?}", create_dup_result);
    }

    #[test]
    fn backend_create_open() {
        use super::super::super::storage::Storage;
        use url::Url;

        let sync_backend = sync_backend_fixture();
        let storage = Storage::new(
            sync_backend.clone(),
            Url::parse(format!("http://{}/{}", TEST_HOST, TEST_BASE_PATH).as_ref()).unwrap());

        {
            let mut backend = sync_backend.0.write().unwrap();
            let co_result = backend.create_open(TEST_DOMAIN, "test/key/3", &storage);
            assert!(co_result.is_ok());
            let urls = co_result.unwrap();
            assert_eq!(1, urls.len());
            assert_eq!(
                Url::parse(format!("http://{}/{}/d/{}/k/{}", TEST_HOST, TEST_BASE_PATH, TEST_DOMAIN, "test/key/3").as_ref()).unwrap(),
                urls[0]);
        }

        {
            let backend = sync_backend.0.read().unwrap();
            let file = backend.file(TEST_DOMAIN, "test/key/3");
            assert!(matches!(file, Ok(Some(..))), "Create opened file was {:?}", file);
            let file = file.unwrap().unwrap();
            assert_eq!("test/key/3", file.key());
            assert!(file.content.is_none());
            assert!(file.size.is_none());
        }

        {
            let mut backend = sync_backend.0.write().unwrap();
            let co_result = backend.create_open(TEST_DOMAIN, TEST_KEY_1, &storage);
            assert!(co_result.is_ok(), "Create open with duplicate key result was {:?}", co_result);
            let urls = co_result.unwrap();
            assert_eq!(1, urls.len());
            assert_eq!(
                Url::parse(format!("http://{}/{}/d/{}/k/{}", TEST_HOST, TEST_BASE_PATH, TEST_DOMAIN, TEST_KEY_1).as_ref()).unwrap(),
                urls[0]);
        }

        // {
        //     let mut backend = sync_backend.0.lock().unwrap();
        //     let co_result = backend.create_open("test_domain_2", "test/key/3", &storage);
        //     assert!(
        //         matches!(co_result, Err(MogError::UnregDomain(ref k)) if k == "test_domain_2"),
        //         "Create open with unknown domain result was {:?}", co_result);
        // }
    }

    #[test]
    fn domain_list_keys() {
        let backend = backend_fixture();
        let list_result = backend.list_keys(TEST_DOMAIN, None, None, None);
        assert!(list_result.is_ok());
        assert_eq!(vec![ TEST_KEY_1, TEST_KEY_2 ], list_result.unwrap());
    }

    #[test]
    fn domain_list_keys_limit() {
        let backend = full_backend_fixture();
        let list_result = backend.list_keys(TEST_FULL_DOMAIN, None, None, Some(10));
        assert!(list_result.is_ok());
        let list = list_result.unwrap();
        assert_eq!(10, list.len());
        assert!(list[0] < list[9]);
    }

    #[test]
    fn domain_list_keys_after() {
        let backend = full_backend_fixture();
        let first_list = backend.list_keys(TEST_FULL_DOMAIN, None, None, Some(10)).unwrap();
        let after_key = first_list.iter().last().unwrap();

        let list_result = backend.list_keys(TEST_FULL_DOMAIN, None, Some(after_key), None);
        assert!(list_result.is_ok());
        let list = list_result.unwrap();
        assert!(after_key < &list[0]);
        assert!(&list[0] < list.iter().last().unwrap());
    }

    #[test]
    fn domain_list_keys_prefix() {
        let backend = full_backend_fixture();
        let list_result = backend.list_keys(TEST_FULL_DOMAIN, Some(TEST_KEY_PREFIX_1), None, None);
        assert!(list_result.is_ok());
        let list = list_result.unwrap();
        for key in list.iter() {
            assert!(key.starts_with(TEST_KEY_PREFIX_1), "key {:?} doesn't start with {:?}", key, TEST_KEY_PREFIX_1);
        }
    }

    #[test]
    fn domain_delete_key() {
        let mut backend = backend_fixture();

        {
            let delete_result = backend.delete(TEST_DOMAIN, TEST_KEY_1);
            assert!(matches!(delete_result, Ok(())));
        }

        assert!(backend.domains[TEST_DOMAIN].file(TEST_KEY_1).is_none());

        {
            let delete_result_2 = backend.delete(TEST_DOMAIN, TEST_KEY_1);
            assert!(matches!(delete_result_2, Err(MogError::UnknownKey(ref k)) if k == TEST_KEY_1))
        }
    }
}

#[cfg(test)]
pub mod test_support {
    use std::collections::HashMap;
    use super::*;
    use super::super::Domain;
    use super::super::model::test_support::{domain_fixture, full_domain_fixture};

    pub fn backend_fixture() -> MemBackend {
        let mut backend = MemBackend {
            domains: HashMap::new(),
            empty_domain: Domain::new(""),
        };
        let domain = domain_fixture();
        backend.domains.insert(domain.name().to_string(), domain);
        backend
    }

    pub fn full_backend_fixture() -> MemBackend {
        let mut backend = MemBackend {
            domains: HashMap::new(),
            empty_domain: Domain::new(""),
        };
        let domain = full_domain_fixture();
        backend.domains.insert(domain.name().to_string(), domain);
        backend
    }

    pub fn sync_backend_fixture() -> SyncMemBackend {
        SyncMemBackend::new(backend_fixture())
    }
}
