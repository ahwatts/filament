use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use super::strings::STRINGS;

// #[derive(Debug)]
// pub struct Domain {
//     name: String,
//     classes: HashMap<String, Class>,
//     files: HashMap<String, FileInfo>,
// }

// impl Domain {
//     pub fn new(name: &str) -> Domain {
//         let mut rv = Domain {
//             name: name.to_string(),
//             classes: HashMap::new(),
//             files: HashMap::new(),
//         };
//         let default_class_name = "default".to_string();
//         Class::new("default", &rv).unwrap();
//         rv
//     }

//     pub fn name(&self) -> &str {
//         &self.name
//     }

//     pub fn class(&self, name: &str) -> Option<&Class> {
//         self.classes.get(name).map(|c| c.deref())
//     }

//     pub fn classes(&self) -> &[&Class] {
//         let mut rv: Vec<&Class> = self.classes.values().map(|c| c.deref()).collect();
//         // rv.sort_by(|c1, c2| c1.name.cmp(&c2.name));
//         &rv
//     }

//     pub fn file(&self, key: &str) -> Option<&FileInfo> {
//         self.files.get(key)
//     }

//     pub fn file_mut(&mut self, key: &str) -> Option<&mut FileInfo> {
//         self.files.get_mut(key)
//     }

//     pub fn add_file(&mut self, key: &str, info: FileInfo) -> MogResult<&FileInfo> {
//         if self.files.contains_key(key) {
//             Err(MogError::DuplicateKey)
//         } else {
//             self.files.insert(key.to_string(), info);
//             Ok(self.file(key).unwrap())
//         }
//     }

//     pub fn remove_file(&mut self, key: &str) -> Option<FileInfo> {
//         self.files.remove(key)
//     }
// }

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
    pub fn new(key: &str, domain_name: &str, class_name: &str) -> MogResult<FileInfo> {
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
    // domains: HashMap<String, Arc<RefCell<Domain>>>,
    pub files: HashMap<String, FileInfo>,
}

impl Backend {
    pub fn new() -> Backend {
        Default::default()
    }

    // pub fn add_domain(&mut self, name: &str) -> MogResult<&Domain> {
    //     if self.domains.contains_key(name) {
    //         Err(MogError::DuplicateDomain)
    //     } else {
    //         self.domains.insert(name.to_string(), RefCell::new(Domain::new(name)));
    //         Ok(self.domains.get(name).map(|d| d.deref()).unwrap())
    //     }
    // }

    // pub fn domain(&self, name: &str) -> Option<&Domain> {
    //     self.domains.get(name).map(|d| d.deref())
    // }
}

pub type SyncBackend = Arc<Mutex<Backend>>;

pub enum MogError {
    DuplicateKey,
    DuplicateClass,
    DuplicateDomain,
    UnknownClass,
}

pub type MogResult<T> = Result<T, MogError>;

