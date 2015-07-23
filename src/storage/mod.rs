use std::io::{self, Cursor, Read, Write};
use super::common::SyncBackend;
use super::error::{MogError, MogResult};
use url::Url;

pub use self::iron::StorageHandler;

pub mod iron;

#[derive(Clone, Debug)]
pub struct Storage {
    pub base_url: Url,
    backend: SyncBackend,
}

impl Storage {
    pub fn new(backend: SyncBackend, base_url: Url) -> Storage {
        Storage {
            base_url: base_url,
            backend: backend,
        }
    }

    pub fn url_for_key(&self, domain: &str, key: &str) -> Url {
        let mut key_url = self.base_url.clone();
        key_url.path_mut().unwrap().extend([ "d", domain, "k" ].iter().map(|s| s.to_string()));
        key_url.path_mut().unwrap().extend(key.split("/").map(|s| s.to_string()));
        key_url
    }

    pub fn store_content<R: Read>(&self, domain: &str, key: &str, reader: &mut R) -> MogResult<()> {
        // We don't need the lock to do this part...
        let mut content = vec![];
        try!(io::copy(reader, &mut content));

        self.backend.with_file_mut(domain, key, move|file_info| {
            file_info.content = Some(content);
            Ok(())
        })
    }

    pub fn get_content<W: Write>(&self, domain: &str, key: &str, writer: &mut W) -> MogResult<()> {
        self.backend.with_file(domain, key, move|file_info| {
            match file_info.content {
                Some(ref reader) => {
                    try!(io::copy(&mut Cursor::new(reader.as_ref()), writer));
                    Ok(())
                },
                None => Err(MogError::NoContent(key.to_string())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::super::error::MogError;
    use super::super::test_support::*;

    #[test]
    fn url_for_key() {
        let storage = storage_fixture();
        assert_eq!(
            format!("http://{}/{}/d/{}/k/{}", TEST_HOST, TEST_BASE_PATH, TEST_DOMAIN, TEST_KEY_1),
            storage.url_for_key(TEST_DOMAIN, TEST_KEY_1).serialize());
    }

    #[test]
    fn get_content() {
        let storage = storage_fixture();
        let mut content = vec![];

        storage.get_content(TEST_DOMAIN, TEST_KEY_1, &mut content).unwrap_or_else(|e| {
            panic!("Error retrieving content from {:?}: {}", TEST_KEY_1, e);
        });

        let content_ref: &[u8] = &content;
        assert_eq!(TEST_CONTENT_1, content_ref);
    }

    #[test]
    fn get_content_unknown_key() {
        let storage = storage_fixture();
        let mut content = vec![];
        assert!( matches!(storage.get_content(TEST_DOMAIN, "test/key/3", &mut content).unwrap_err(),
                          MogError::UnknownKey(ref k) if k == "test/key/3"));
        assert!(content.is_empty());
    }

    #[test]
    fn get_content_no_content() {
        let storage = storage_fixture();
        let mut content = vec![];
        assert!(matches!(storage.get_content(TEST_DOMAIN, TEST_KEY_2, &mut content).unwrap_err(),
                         MogError::NoContent(ref k) if k == TEST_KEY_2));
        assert!(content.is_empty());
    }

    #[test]
    fn store_replace_content() {
        let storage = storage_fixture();
        let new_content = Vec::from("This is new test content");

        storage.store_content(TEST_DOMAIN, TEST_KEY_1, &mut Cursor::new(new_content.clone())).unwrap_or_else(|e| {
            panic!("Error storing content to {:?}: {}", TEST_KEY_1, e);
        });

        storage.backend.with_file(TEST_DOMAIN, TEST_KEY_1, move|file| {
            assert_eq!(&new_content, file.content.as_ref().unwrap());
            Ok(())
        }).unwrap();
    }

    #[test]
    fn store_new_content() {
        let storage = storage_fixture();
        let new_content = Vec::from("This is new test content");

        storage.store_content(TEST_DOMAIN, TEST_KEY_2, &mut Cursor::new(new_content.clone())).unwrap_or_else(|e| {
            panic!("Error storing content to {:?}: {}", TEST_KEY_2, e);
        });

        storage.backend.with_file(TEST_DOMAIN, TEST_KEY_2, move|file| {
            assert_eq!(&new_content, file.content.as_ref().unwrap());
            Ok(())
        }).unwrap();
    }

    #[test]
    fn store_content_to_unknown_key() {
        let storage = storage_fixture();
        let new_content: &'static [u8] = b"This is new test content";
        assert!(matches!(storage.store_content(TEST_DOMAIN, "test/key/3", &mut Cursor::new(new_content)).unwrap_err(),
                         MogError::UnknownKey(ref k) if k == "test/key/3"));
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use super::super::common::test_support::sync_backend_fixture;
    use url::Url;

    pub static TEST_HOST: &'static str = "test.host";
    pub static TEST_BASE_PATH: &'static str = "base_path";

    pub fn storage_fixture() -> Storage {
        let base_url = Url::parse(&format!("http://{}/{}", TEST_HOST, TEST_BASE_PATH)).unwrap();
        Storage::new(sync_backend_fixture(), base_url)
    }
}
