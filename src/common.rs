use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Domain {
    name: String,
    files: HashMap<String, FileInfo>,
}

impl Domain {
    pub fn new(name: &str) -> Domain {
        Domain {
            name: name.to_string(),
            files: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn file(&self, key: &str) -> Option<&FileInfo> {
        self.files.get(key)
    }

    pub fn file_mut(&mut self, key: &str) -> Option<&mut FileInfo> {
        self.files.get_mut(key)
    }

    pub fn add_file(&mut self, key: &str, info: FileInfo) -> MogResult<&FileInfo> {
        if self.files.contains_key(key) {
            Err(MogError::DuplicateKey)
        } else {
            self.files.insert(key.to_string(), info);
            Ok(self.file(key).unwrap())
        }
    }

    pub fn remove_file(&mut self, key: &str) -> Option<FileInfo> {
        self.files.remove(key)
    }
}

#[derive(Debug)]
pub struct FileInfo {
    key: String,
    pub content: Option<Vec<u8>>,
    pub size: Option<usize>,
}

impl FileInfo {
    pub fn new(key: &str) -> MogResult<FileInfo> {
        Ok(FileInfo {
            key: key.to_string(),
            content: None,
            size: None,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

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
            .ok_or(MogError::UnknownDomain)
            .map(|d| d.file(key))
    }

    pub fn file_mut(&mut self, domain_name: &str, key: &str) -> MogResult<Option<&mut FileInfo>> {
        self.domains.get_mut(domain_name)
            .ok_or(MogError::UnknownDomain)
            .map(|d| d.file_mut(key))
    }
}

pub type SyncBackend = Arc<Mutex<Backend>>;

#[derive(Debug, PartialEq, Eq)]
pub enum MogError {
    DuplicateKey,
    DuplicateClass,
    DuplicateDomain,
    UnknownClass,
    UnknownDomain,
}

pub type MogResult<T> = Result<T, MogError>;

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_support::*;

    #[test]
    fn test_create_domain() {
        let domain = Domain::new("test_domain_2");
        assert_eq!("test_domain_2", domain.name());
        assert!(domain.files.is_empty());
    }

    #[test]
    fn test_domain_get_file() {
        let mut domain = domain_fixture();

        {   // immutable, file present
            let file = domain.file(TEST_KEY_1);
            assert!(file.is_some());
            assert_eq!(TEST_KEY_1, file.unwrap().key());
        }

        {   // immutable, file not present
            let file2 = domain.file("test/key/3");
            assert!(file2.is_none());
        }

        {   // mutable, file present
            let file3 = domain.file_mut(TEST_KEY_1);
            assert!(file3.is_some());
            assert_eq!(TEST_KEY_1, file3.unwrap().key());
        }

        {   // mutable, file not present
            let file4 = domain.file_mut("test/key/3");
            assert!(file4.is_none());
        }
    }

    #[test]
    fn test_domain_get_mut_file() {
        let mut domain = domain_fixture();
        let new_content: Vec<u8> = b"Different content".iter().cloned().collect();

        {   // Modify the content of the file.
            let mut_file = domain.file_mut(TEST_KEY_1).unwrap();
            mut_file.content = Some(new_content.clone());
        }

        {   // Pull it back out and make sure that it's the same.
            let file = domain.file(TEST_KEY_1).unwrap();
            assert_eq!(Some(new_content.clone()), file.content);
        }
    }

    #[test]
    fn test_domain_add_file() {
        let mut domain = domain_fixture();
        let new_key = "test/key/3";
        let content: Vec<u8> = b"New file content".iter().cloned().collect();

        {   // Add a new file to the domain.
            let mut file = FileInfo::new(new_key).unwrap();
            file.content = Some(content.clone());
            file.size = Some(content.len());
            domain.add_file(new_key, file).unwrap();
        }

        {   // Pull it back out and make sure it's the same.
            let file = domain.file(new_key);
            assert!(file.is_some());
            let file = file.unwrap();
            assert_eq!(new_key, file.key());
            assert_eq!(Some(&content), file.content.as_ref());
            assert_eq!(Some(&content.len()), file.size.as_ref());
        }

        {   // Try adding a duplicate key to the domain.
            let file = FileInfo::new(TEST_KEY_1).unwrap();
            let result = domain.add_file(TEST_KEY_1, file);
            assert!(result.is_err());
            assert_eq!(MogError::DuplicateKey, result.unwrap_err());
        }
    }

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

    pub static TEST_DOMAIN: &'static str = "test_domain";

    pub static TEST_HOST: &'static str = "test.host";
    pub static TEST_BASE_PATH: &'static str = "base_path";

    pub static TEST_KEY_1: &'static str = "test/key/1";
    pub static TEST_CONTENT_1: &'static [u8] = b"This is test content";

    pub static TEST_KEY_2: &'static str = "test/key/2";

    pub fn domain_fixture() -> Domain {
        let mut domain = Domain::new(TEST_DOMAIN);
        domain.files.insert(TEST_KEY_1.to_string(), file_1_fixture());
        domain.files.insert(TEST_KEY_2.to_string(), file_2_fixture());
        domain
    }

    pub fn file_1_fixture() -> FileInfo {
        FileInfo {
            key: TEST_KEY_1.to_string(),
            content: Some(Vec::from(TEST_CONTENT_1)),
            size: Some(TEST_CONTENT_1.len()),
        }
    }

    pub fn file_2_fixture() -> FileInfo {
        FileInfo {
            key: TEST_KEY_2.to_string(),
            content: None,
            size: None,
        }
    }

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
