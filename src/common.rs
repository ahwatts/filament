use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Domain {
    name: String,
    // classes: HashMap<String, Class>,
    files: HashMap<String, FileInfo>,
}

impl Domain {
    pub fn new(name: &str) -> Domain {
        let rv = Domain {
            name: name.to_string(),
            // classes: HashMap::new(),
            files: HashMap::new(),
        };
        // let default_class_name = "default".to_string();
        // Class::new("default", &rv).unwrap();
        rv
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    // pub fn class(&self, name: &str) -> Option<&Class> {
    //     self.classes.get(name).map(|c| c.deref())
    // }
    //
    // pub fn classes(&self) -> &[&Class] {
    //     let mut rv: Vec<&Class> = self.classes.values().map(|c| c.deref()).collect();
    //     // rv.sort_by(|c1, c2| c1.name.cmp(&c2.name));
    //     &rv
    // }

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

// #[derive(Debug)]
// pub struct Class {
//     name: String,
// }

// impl Class {
//     pub fn new(name: &str, domain: &mut Domain) -> MogResult<Class> {
//         if domain.classes.contains_key(name) {
//             Err(MogError::DuplicateClass)
//         } else {
//             let class_name = name.to_string();
//             let rv = Class { name: class_name.clone() };
//             domain.classes.insert(class_name, rv);
//             Ok(rv)
//         }
//     }

//     pub fn name(&self) -> &str {
//         &self.name
//     }
// }

#[derive(Debug)]
pub struct FileInfo {
    key: String,
    pub content: Option<Vec<u8>>,
    pub size: Option<usize>,
    // domain_name: &'static str,
    // class_name: &'static str,
}

impl FileInfo {
    pub fn new(key: &str) -> MogResult<FileInfo> {
        Ok(FileInfo {
            key: key.to_string(),
            content: None,
            size: None,
            // domain_name: domain_name,
            // class_name: class_name,
        })
        // match domain.classes.get(class_name) {
        //     Some(class) => Ok(FileInfo {
        //         key: key.to_string(),
        //         content: None,
        //         size: None,
        //         domain: domain_name,
        //         class: class_name,
        //     }),
        //     None => Err(MogError::UnknownClass)
        // }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    // pub fn domain(&self) -> &Domain {
    //     &self.domain
    // }

    // pub fn class(&self) -> &Class {
    //     &self.class
    // }
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

#[derive(Debug)]
pub enum MogError {
    DuplicateKey,
    DuplicateClass,
    DuplicateDomain,
    UnknownClass,
    UnknownDomain,
}

pub type MogResult<T> = Result<T, MogError>;

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

    pub fn backend_fixture() -> Backend {
        let mut domain = Domain::new(TEST_DOMAIN);

        domain.files.insert(
            TEST_KEY_1.to_string(),
            FileInfo {
                key: TEST_KEY_1.to_string(),
                content: Some(Vec::from(TEST_CONTENT_1)),
                size: Some(TEST_CONTENT_1.len()),
            });

        domain.files.insert(
            TEST_KEY_2.to_string(),
            FileInfo {
                key: TEST_KEY_2.to_string(),
                content: None,
                size: None,
            });

        let mut backend = Backend {
            domains: HashMap::new(),
        };
        backend.domains.insert(domain.name.clone(), domain);
        backend
    }

    pub fn sync_backend_fixture() -> SyncBackend {
        Arc::new(Mutex::new(backend_fixture()))
    }
}
