use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Domain {
    name: String,
    classes: HashMap<String, Arc<Class>>,
    files: HashMap<String, FileInfo>,
}

impl Domain {
    pub fn new(name: &str) -> Arc<Domain> {
        let rv = Arc::new(Domain {
            name: name.to_string(),
            classes: HashMap::new(),
            files: HashMap::new(),
        });
        let default_class_name = "default".to_string();

        rv.classes.insert(default_class_name.clone(), Arc::new(Class {
            name: default_class_name,
            domain: rv.clone(),
        }));

        rv
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn class(&self, name: &str) -> Option<&Class> {
        self.classes.get(name).map(|c| c.deref())
    }

    pub fn classes(&self) -> &[&Class] {
        let mut rv: Vec<&Class> = self.classes.values().map(|c| c.deref()).collect();
        rv.sort_by(|c1, c2| c1.name.cmp(&c2.name));
        &rv
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
pub struct Class {
    name: String,
    domain: Arc<Domain>,
}

impl Class {
    pub fn new(name: &str, domain: Arc<Domain>) -> MogResult<Arc<Class>> {
        if domain.classes.contains_key(name) {
            Err(MogError::DuplicateClass)
        } else {
            let class_name = name.to_string();
            let rv = Arc::new(Class { name: class_name.clone(), domain: domain });
            domain.classes.insert(class_name, rv);
            Ok(rv)
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn domain(&self) -> &Domain {
        &self.domain
    }
}

#[derive(Debug)]
pub struct FileInfo {
    key: String,
    content: Option<Vec<u8>>,
    size: Option<usize>,
    domain: Arc<Domain>,
    class: Arc<Class>,
}

impl FileInfo {
    pub fn new(key: &str, domain: Arc<Domain>, class_name: &str) -> MogResult<FileInfo> {
        match domain.classes.get(class_name) {
            Some(class) => Ok(FileInfo {
                key: key.to_string(),
                content: None,
                size: None,
                domain: domain,
                class: class.clone(),
            }),
            None => Err(MogError::UnknownClass)
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn domain(&self) -> &Domain {
        &self.domain
    }

    pub fn class(&self) -> &Class {
        &self.class
    }
}

#[derive(Debug, Default)]
pub struct Backend {
    files: HashMap<String, Arc<FileInfo>>,
    domains: HashMap<String, Arc<Domain>>,
    classes: HashMap<String, Arc<Class>>,
}

impl Backend {
    pub fn new() -> Backend {
        Default::default()
    }

    // pub fn add_domain(&mut self, name: &str) -> Arc<Domain> {
    //     self.domains.entry(name).or_insert(Domain::new(name))
    // }
}

pub type SyncBackend = Arc<Mutex<Backend>>;

pub enum MogError {
    DuplicateKey,
    DuplicateClass,
    UnknownClass,
}

pub type MogResult<T> = Result<T, MogError>;

