#![allow(unused_variables, dead_code)]

//! Utilities for working with the MogileFS database.

use chrono::UTC;
use mysql::conn::{MyOpts, MyConn, QueryResult};
use mysql::value::{self, ToRow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

pub struct DataStore {
    conn: RefCell<MyConn>,
    domain_cache: RefCell<ObjectCache<Domain>>,
    class_cache: RefCell<ObjectCache<Class>>,
}

impl DataStore {
    pub fn new(opts: MyOpts) -> Result<DataStore, String> {
        MyConn::new(opts).map(|c| {
            DataStore {
                conn: RefCell::new(c),
                domain_cache: RefCell::new(ObjectCache::new()),
                class_cache: RefCell::new(ObjectCache::new()),
            }
        }).map_err(|e| {
            format!("Error connecting to database: {}", e)
        })
    }

    pub fn domain_by_id(&self, dmid: u16) -> Option<Rc<Domain>> {
        let mut domain = self.domain_cache.borrow().find_by_id(dmid as usize);
        domain.clone().or_else(|| {
            let query_result = self.select("SELECT dmid, namespace FROM domain WHERE dmid = ?", (dmid,), |result| {
                if let Some(Ok(db_row)) = result.next() {
                    let (new_dmid, new_name) = value::from_row::<(u16, String)>(db_row);
                    let db_domain = Domain { dmid: new_dmid, name: new_name.clone() };
                    self.domain_cache.borrow_mut().add(db_domain, new_dmid as usize, new_name);
                    domain = self.domain_cache.borrow().find_by_id(dmid as usize);
                }
            });

            domain
        })
    }

    pub fn domain_by_name(&self, name: &str) -> Option<Rc<Domain>> {
        let mut domain = self.domain_cache.borrow().find_by_name(name);
        let mut found = false;
        domain.clone().or_else(|| {
            self.select("SELECT dmid, namespace FROM domain WHERE namespace = ?", (name,), |result| {
                if let Some(Ok(db_row)) = result.next() {
                    found = true;
                    let (new_dmid, new_name) = value::from_row::<(u16, String)>(db_row);
                    let db_domain = Domain { dmid: new_dmid, name: new_name.clone() };
                    self.domain_cache.borrow_mut().add(db_domain, new_dmid as usize, new_name);
                    domain = self.domain_cache.borrow().find_by_name(name);
                }
            });

            domain
        })
    }

    fn select<Q: AsRef<str>, P: ToRow, F>(&self, query: Q, params: P, callback: F)
        where F: FnOnce(&mut QueryResult)
    {
        let mut conn = self.conn.borrow_mut();
        trace!("Executing query: {:?}", query.as_ref());
        let start = UTC::now();
        let result = conn.prep_exec(query.as_ref(), params);
        let end = UTC::now();
        trace!("Query took {:?}", end - start);

        match result {
            Ok(mut result_set) => {
                debug!("Select query ({:?}): {:?}", end - start, query.as_ref());
                callback(&mut result_set);
            },
            Err(e) => {
                error!("Error with query {:?}: {}", query.as_ref(), e);
            }
        }
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
            by_id: vec![None, None, None, None, None],
            by_name: HashMap::new(),
        }
    }

    fn add(&mut self, object: T, id: usize, name: String) {
        let object_rc = Rc::new(object);

        println!("id = {} self.by_id.len() = {}", id, self.by_id.len());

        if self.by_id.len() <= id {
            let needed = self.by_id.len() - id + 1;
            self.by_id.extend(iter::repeat(None).take(needed));
        }

        self.by_id[id] = Some(object_rc.clone());
        self.by_name.entry(name).or_insert(object_rc.clone());
    }

    fn find_by_id(&self, id: usize) -> Option<Rc<T>> {
        self.by_id.get(id).and_then(|o| o.clone())
    }

    fn find_by_name(&self, name: &str) -> Option<Rc<T>> {
        self.by_name.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use mysql::conn::MyOpts;
    use std::default::Default;
    use std::env;
    use std::io::{self, Write};
    use std::net::ToSocketAddrs;
    use std::str::FromStr;
    use super::*;

    lazy_static!{
        static ref FILAMENT_TEST_DB_HOST: String = env::var("FILAMENT_TEST_DB_HOST").ok().unwrap_or("127.0.0.1:3306".to_string());
        static ref FILAMENT_TEST_DB_USER: String = env::var("FILAMENT_TEST_DB_USER").ok().unwrap_or("gibberish".to_string());
        static ref FILAMENT_TEST_DB_PASS: String = env::var("FILAMENT_TEST_DB_PASS").ok().unwrap_or("gobbledegook".to_string());
        static ref FILAMENT_TEST_DB_NAME: String = env::var("FILAMENT_TEST_DB_NAME").ok().unwrap_or("mogilefs".to_string());
        static ref FILAMENT_TEST_DOMAIN_ID: u16 = u16::from_str(&env::var("FILAMENT_TEST_DOMAIN_ID").ok().unwrap_or("1".to_string())).unwrap();
        static ref FILAMENT_TEST_DOMAIN_NAME: String = env::var("FILAMENT_TEST_DOMAIN_NAME").ok().unwrap_or("filament_test".to_string());
    }

    fn data_store_fixture() -> Result<DataStore, String> {
        let host_sock_addr = FILAMENT_TEST_DB_HOST.to_socket_addrs().unwrap().next().unwrap();
        DataStore::new(MyOpts {
            tcp_addr: Some(format!("{}", host_sock_addr).split(":").next().unwrap().to_owned()),
            tcp_port: host_sock_addr.port(),
            user: Some(FILAMENT_TEST_DB_USER.clone()),
            pass: Some(FILAMENT_TEST_DB_PASS.clone()),
            db_name: Some(FILAMENT_TEST_DB_NAME.clone()),
            ..Default::default()
        })
    }

    fn skip() {
        write!(&mut io::stdout(), "(skipped) ").unwrap();
    }

    macro_rules! test_store {
        () => {
            {
                match data_store_fixture() {
                    Ok(s) => s,
                    Err(e) => {
                        println!("Could not connect to DB: {}", e);
                        skip();
                        return;
                    }
                }
            }
        }
    }

    #[test]
    fn test_get_domain_by_name() {
        let store = test_store!();
        let domain = store.domain_by_name(&*FILAMENT_TEST_DOMAIN_NAME).unwrap();
        assert_eq!(*FILAMENT_TEST_DOMAIN_ID, domain.dmid);
        assert_eq!(*FILAMENT_TEST_DOMAIN_NAME, domain.name);
    }

    #[test]
    fn test_get_domain_by_id() {
        let store = test_store!();
        let domain = store.domain_by_id(*FILAMENT_TEST_DOMAIN_ID).unwrap();
        assert_eq!(*FILAMENT_TEST_DOMAIN_ID, domain.dmid);
        assert_eq!(*FILAMENT_TEST_DOMAIN_NAME, domain.name);
    }

    #[test]
    fn test_get_missing_domain() {
        let store = test_store!();
        let domain = store.domain_by_name("This domain doesn't exist!");
        assert!(domain.is_none());

        let domain2 = store.domain_by_id(65534);
        assert!(domain2.is_none());
    }
}
