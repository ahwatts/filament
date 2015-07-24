use std::collections::BTreeMap;
use std::collections::btree_map;
use std::iter::Iterator;
#[allow(unused_imports)] use super::super::error::{MogError, MogResult};

#[derive(Debug)]
pub struct Domain {
    name: String,
    files: BTreeMap<String, FileInfo>,
}

impl Domain {
    pub fn new(name: &str) -> Domain {
        Domain {
            name: name.to_string(),
            files: BTreeMap::new(),
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

    pub fn files<'a>(&'a self) -> Files<'a> {
        Files { inner: self.files.iter(), }
    }

    pub fn add_file(&mut self, key: &str, info: FileInfo) -> MogResult<&FileInfo> {
        self.files.insert(key.to_string(), info);
        Ok(self.file(key).unwrap())
    }

    pub fn remove_file(&mut self, key: &str) -> Option<FileInfo> {
        self.files.remove(key)
    }

    // pub fn rename(&mut self, from: &str, to: &str) -> MogResult<()> {
    // }
}

pub struct Files<'a> {
    inner: btree_map::Iter<'a, String, FileInfo>,
}

impl<'a> Iterator for Files<'a> {
    type Item = (&'a str, &'a FileInfo);

    fn next(&mut self) -> Option<(&'a str, &'a FileInfo)> {
        self.inner.next().map(|(k, v)| (k.as_ref(), v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
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
    use super::super::super::test_support::*;
    #[allow(unused_imports)] use super::super::super::error::MogError;

    #[test]
    fn create_domain() {
        let domain = Domain::new("test_domain_2");
        assert_eq!("test_domain_2", domain.name());
        assert!(domain.files.is_empty());
    }

    #[test]
    fn domain_get_file() {
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
    fn domain_get_mut_file() {
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
    fn domain_list_files() {
        let domain = domain_fixture();
        let mut files = domain.files();

        let file_1 = files.next();
        assert_eq!(Some(TEST_KEY_1), file_1.map(|(k, _)| k));
        assert_eq!(Some(TEST_KEY_1), file_1.map(|(_, fi)| fi.key()));

        let file_2 = files.next();
        assert_eq!(Some(TEST_KEY_2), file_2.map(|(k, _)| k));
        assert_eq!(Some(TEST_KEY_2), file_2.map(|(_, fi)| fi.key()));

        assert!(files.next().is_none());
    }

    #[test]
    fn domain_add_file() {
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

        {   // Try adding a duplicate key to the domain, which should create a new empty file.
            let file = FileInfo::new(TEST_KEY_1);
            let result = domain.add_file(TEST_KEY_1, file);
            assert!(result.is_ok());
            let file = result.unwrap();
            assert_eq!(TEST_KEY_1, file.key());
            assert_eq!(None, file.content);
            assert_eq!(None, file.size);
        }
    }

    #[test]
    fn domain_remove_file() {
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
}

#[cfg(test)]
pub mod test_support {
    use super::*;

    pub static TEST_DOMAIN: &'static str = "test_domain";
    pub static TEST_KEY_1: &'static str = "test/key/1";
    pub static TEST_KEY_2: &'static str = "test/key/2";
    pub static TEST_CONTENT_1: &'static [u8] = b"This is test content";

    pub static TEST_FULL_DOMAIN: &'static str = "test_full_domain";
    pub static TEST_KEY_PREFIX_1: &'static str = "foo/prefix";
    pub static TEST_KEY_PREFIX_2: &'static str = "bar/prefix";
    pub static TEST_PREFIX_COUNT: u32 = 100;

    pub fn domain_fixture() -> Domain {
        let mut domain = Domain::new(TEST_DOMAIN);
        domain.files.insert(TEST_KEY_1.to_string(), file_1_fixture());
        domain.files.insert(TEST_KEY_2.to_string(), file_2_fixture());
        domain
    }

    pub fn full_domain_fixture() -> Domain {
        let mut domain = Domain::new(TEST_FULL_DOMAIN);
        for i in (0..TEST_PREFIX_COUNT) {
            let key_p1 = format!("{}/key/{}", TEST_KEY_PREFIX_1, i+1);
            let key_p2 = format!("{}/key/{}", TEST_KEY_PREFIX_2, i+1);

            domain.files.insert(key_p1.clone(), FileInfo {
                key: key_p1,
                content: None,
                size: None,
            });

            domain.files.insert(key_p2.clone(), FileInfo {
                key: key_p2,
                content: None,
                size: None,
            });
        }

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
