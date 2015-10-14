#![allow(unused_variables, dead_code)]

//! Utilities for working with the MogileFS database.

use mysql::conn::{MyOpts, MyConn};
use std::collections::HashMap;
use std::rc::Rc;

pub struct DataStore {
    conn: MyConn,
    domain_cache: ObjectCache<Domain>,
    class_cache: ObjectCache<Class>,
}

impl DataStore {
    pub fn new(opts: MyOpts) -> Result<DataStore, String> {
        MyConn::new(opts).map(|c| {
            DataStore {
                conn: c,
                domain_cache: ObjectCache::new(),
                class_cache: ObjectCache::new(),
            }
        }).map_err(|e| {
            format!("Error connecting to database: {}", e)
        })
    }

    pub fn domain_by_id(&self, dmid: u16) -> Option<Rc<Domain>> {
        self.domain_cache.find_by_id(dmid as usize).or_else(|| {
            unimplemented!()
        })
    }

    pub fn domain_by_name(&self, name: &str) -> Option<Rc<Domain>> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Domain {
    pub dmid: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub classid: u8,
    pub domain: Rc<Domain>,
    pub name: String,
    pub mindevcount: u8,
    // pub hashtype: u8,
    pub replpolicy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fid {
    pub fid: u64,
    pub domain: Rc<Domain>,
    pub key: String,
    pub length: u64,
    pub class: Rc<Class>,
    pub devcount: u8,
}

#[derive(Debug, Clone)]
struct ObjectCache<T> {
    by_id: Vec<Option<Rc<T>>>,
    by_name: HashMap<String, Rc<T>>,
}

impl<T> ObjectCache<T> {
    fn new() -> ObjectCache<T> {
        ObjectCache {
            by_id: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    fn find_by_id(&self, id: usize) -> Option<Rc<T>> {
        self.by_id.get(id).and_then(|o| o.clone())
    }

    fn find_by_name(&self, name: &str) -> Option<Rc<T>> {
        self.by_name.get(name).cloned()
    }
}
