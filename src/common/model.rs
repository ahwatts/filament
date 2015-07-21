use std::collections::HashMap;
use super::super::error::{MogError, MogResult};

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
            Err(MogError::DuplicateKey(Some(key.to_string())))
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
    pub fn new(key: &str) -> FileInfo {
        FileInfo {
            key: key.to_string(),
            content: None,
            size: None,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_support::*;
    use super::super::super::error::MogError;

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
            let mut file = FileInfo::new(new_key);
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
            let file = FileInfo::new(TEST_KEY_1);
            let result = domain.add_file(TEST_KEY_1, file);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), MogError::DuplicateKey(..)));
        }
    }
}

#[cfg(test)]
pub mod test_support {
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
}
