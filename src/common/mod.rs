use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::error::{MogError, MogResult};

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

    pub fn file(&self, domain_name: &str, key: &str) -> MogResult<Option<&FileInfo>> {
        self.domains.get(domain_name)
            .ok_or(MogError::UnknownDomain(Some(domain_name.to_string())))
            .map(|d| d.file(key))
    }

    pub fn file_mut(&mut self, domain_name: &str, key: &str) -> MogResult<Option<&mut FileInfo>> {
        self.domains.get_mut(domain_name)
            .ok_or(MogError::UnknownDomain(Some(domain_name.to_string())))
            .map(|d| d.file_mut(key))
    }
}

pub type SyncBackend = Arc<Mutex<Backend>>;

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::error::MogError;
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
    use std::sync::{Arc, Mutex};
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
        Arc::new(Mutex::new(backend_fixture()))
    }
}
