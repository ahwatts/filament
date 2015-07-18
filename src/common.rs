use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
pub struct Domain {
    name: String,
    classes: Vec<Arc<Class>>,
}

impl Domain {
    pub fn new(name: &str) -> Arc<Domain> {
        Arc::new(Domain { name: name.to_string(), classes: vec![] })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn classes(&self) -> &[&Class] {
        let rv: Vec<&Class> = self.classes.iter().map(|c| c.deref()).collect();
        &rv
    }
}

#[derive(Debug, PartialEq)]
pub struct Class {
    name: String,
    domain: Arc<Domain>,
}

impl Class {
    pub fn new(name: &str, domain: Arc<Domain>) -> Arc<Class> {
        let rv = Arc::new(Class { name: name.to_string(), domain: domain });
        domain.classes.push(rv.clone());
        rv
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
    pub fn new(key: &str, domain: Arc<Domain>, class: Arc<Class>) -> FileInfo {
        if !domain.classes().contains(&class.deref()) {
            // Probably should handle this more gracefully, maybe in a result?
            panic!("Domain {:?} does not have a class {:?}", domain.name(), class.name());
        }

        FileInfo {
            key: key.to_string(),
            content: None,
            size: None,
            domain: domain,
            class: class,
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

pub struct Backend(pub HashMap<String, FileInfo>);
pub type SyncBackend = Arc<Mutex<Backend>>;
