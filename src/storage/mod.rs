use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Cursor, Read, Write};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use super::common::{Backend, SyncBackend};
use url::Url;

pub mod iron;

pub struct Storage {
    base_url: Url,
    backend: SyncBackend,
}

impl Storage {
    pub fn new(backend: SyncBackend, base_url: Url) -> Storage {
        Storage {
            base_url: base_url,
            backend: backend,
        }
    }

    pub fn url_for_key(&self, key: &str) -> Url {
        let mut key_url = self.base_url.clone();
        key_url.path_mut().unwrap().extend(key.split("/").map(|s| s.to_string()));
        key_url
    }

    pub fn store_content<R: Read>(&self, key: &str, reader: &mut R) -> StorageResult<()> {
        let mut guard = try!(self.backend.lock());
        let &mut Backend(ref mut backend) = guard.deref_mut();

        match backend.get_mut(key) {
            Some(file_info) => {
                let mut content = vec![];
                try!(io::copy(reader, &mut content));
                file_info.content = Some(content);
                Ok(())
            },
            None => Err(StorageError::UnknownKey)
        }
    }

    pub fn get_content<W: Write>(&self, key: &str, writer: &mut W) -> StorageResult<()> {
        let guard = try!(self.backend.lock());
        let &Backend(ref backend) = guard.deref();

        match backend.get(key) {
            Some(ref file_info) => {
                match file_info.content {
                    Some(ref reader) => {
                        try!(io::copy(&mut Cursor::new(reader.as_ref()), writer));
                        Ok(())
                    },
                    None => Err(StorageError::NoContent),
                }
            },
            None => {
                Err(StorageError::UnknownKey)
            }
        }
    }
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    PoisonedMutex,
    UnknownKey,
    NoContent,
}

impl<'a> From<PoisonError<MutexGuard<'a, Backend>>> for StorageError {
    fn from (_: PoisonError<MutexGuard<'a, Backend>>) -> StorageError {
        StorageError::PoisonedMutex
    }
}

impl From<io::Error> for StorageError {
    fn from(io_err: io::Error) -> StorageError {
        StorageError::Io(io_err)
    }
}

impl PartialEq for StorageError {
    fn eq(&self, other: &StorageError) -> bool {
        use self::StorageError::*;

        match (self, other) {
            (&Io(_), &Io(_)) => true,
            (&PoisonedMutex, &PoisonedMutex) => true,
            (&UnknownKey, &UnknownKey) => true,
            (&NoContent, &NoContent) => true,
            _ => false,
        }
    }
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::StorageError::*;
        match *self {
            Io(ref io_err) => write!(f, "{}", io_err),
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl Error for StorageError {
    fn description(&self) -> &str {
        use self::StorageError::*;
        match *self {
            Io(ref io_err) => io_err.description(),
            PoisonedMutex => "Poisoned mutex",
            UnknownKey => "Unknown key",
            NoContent => "No content",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};
    use super::*;
    use super::super::common::FileInfo;
    use super::super::test_support::*;
    use url::Url;

    fn fixture() -> Storage {
        let base_url = Url::parse(&format!("http://{}/{}", TEST_HOST, TEST_BASE_PATH)).unwrap();
        Storage::new(sync_backend_fixture(), base_url)
    }

    #[test]
    fn url_for_key() {
        let storage = fixture();
        assert_eq!(
            format!("http://{}/{}/{}", TEST_HOST, TEST_BASE_PATH, TEST_KEY_1),
            storage.url_for_key(TEST_KEY_1).serialize());
    }

    #[test]
    fn get_content() {
        let storage = fixture();
        let mut content = vec![];

        storage.get_content(TEST_KEY_1, &mut content).unwrap_or_else(|e| {
            panic!("Error retrieving content from {:?}: {}", TEST_KEY_1, e);
        });

        let content_ref: &[u8] = &content;
        assert_eq!(TEST_CONTENT_1, content_ref);
    }

    #[test]
    fn get_content_unknown_key() {
        let storage = fixture();
        let mut content = vec![];
        assert_eq!(StorageError::UnknownKey, storage.get_content("test/key/3", &mut content).unwrap_err());
        assert!(content.is_empty());
    }

    #[test]
    fn get_content_no_content() {
        let storage = fixture();
        let mut content = vec![];
        assert_eq!(StorageError::NoContent, storage.get_content(TEST_KEY_2, &mut content).unwrap_err());
        assert!(content.is_empty());
    }

    #[test]
    fn store_replace_content() {
        let storage = fixture();
        let new_content: &'static [u8] = b"This is new test content";

        storage.store_content(TEST_KEY_1, &mut Cursor::new(new_content)).unwrap_or_else(|e| {
            panic!("Error storing content to {:?}: {}", TEST_KEY_1, e);
        });

        let guard = storage.backend.lock().unwrap();
        let content: &[u8] = guard.0.get(TEST_KEY_1).unwrap().content.as_ref().unwrap();
        assert_eq!(new_content, content);
    }

    #[test]
    fn store_new_content() {
        let storage = fixture();
        let new_content: &'static [u8] = b"This is new test content";

        storage.store_content(TEST_KEY_2, &mut Cursor::new(new_content)).unwrap_or_else(|e| {
            panic!("Error storing content to {:?}: {}", TEST_KEY_2, e);
        });

        let guard = storage.backend.lock().unwrap();
        let content: &[u8] = guard.0.get(TEST_KEY_2).unwrap().content.as_ref().unwrap();
        assert_eq!(new_content, content);
    }

    #[test]
    fn store_content_to_unknown_key() {
        let storage = fixture();
        let new_content: &'static [u8] = b"This is new test content";
        assert_eq!(StorageError::UnknownKey, storage.store_content("test/key/3", &mut Cursor::new(new_content)).unwrap_err());
    }
}
