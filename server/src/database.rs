//! Utilities for working with the MogileFS database.

use chrono::UTC;
use mysql::conn::{MyOpts, MyConn, QueryResult};
use mysql::value::{self, ToRow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

pub struct DataStore {
    conn: RefCell<MyConn>,
    domain_cache: RefCell<ObjectCache<u16, Domain>>,
    class_cache: RefCell<ObjectCache<(u16, u8), Class>>,
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
        let mut domain = self.domain_cache.borrow().find_by_id(&dmid);
        domain.clone().or_else(|| {
            self.select("SELECT dmid, namespace FROM domain WHERE dmid = ?", (dmid,), |result| {
                if let Some(Ok(db_row)) = result.next() {
                    let (new_dmid, new_name) = value::from_row::<(u16, String)>(db_row);
                    let db_domain = Domain { dmid: new_dmid, name: new_name.clone() };
                    self.domain_cache.borrow_mut().add(db_domain, new_dmid, new_name);
                    domain = self.domain_cache.borrow().find_by_id(&dmid);
                }
            });

            domain
        })
    }

    pub fn domain_by_name(&self, name: &str) -> Option<Rc<Domain>> {
        let mut domain = self.domain_cache.borrow().find_by_name(name);
        domain.clone().or_else(|| {
            self.select("SELECT dmid, namespace FROM domain WHERE namespace = ?", (name,), |result| {
                if let Some(Ok(db_row)) = result.next() {
                    let (new_dmid, new_name) = value::from_row::<(u16, String)>(db_row);
                    let db_domain = Domain { dmid: new_dmid, name: new_name.clone() };
                    self.domain_cache.borrow_mut().add(db_domain, new_dmid, new_name);
                    domain = self.domain_cache.borrow().find_by_name(name);
                }
            });

            domain
        })
    }

    pub fn class_by_id(&self, dmid: u16, classid: u8) -> Option<Rc<Class>> {
        let mut class = self.class_cache.borrow().find_by_id(&(dmid, classid));
        class.clone().or_else(|| {
            self.select("SELECT dmid, classid, classname, mindevcount, hashtype, replpolicy FROM class WHERE dmid = ? and classid = ?", (dmid, classid), |result| {
                if let Some(Ok(db_row)) = result.next() {
                    let (new_dmid, new_classid, classname, mindevcount, _hashtype, replpolicy) =
                        value::from_row::<(u16, u8, String, u8, u8, String)>(db_row);
                    let db_class = Class {
                        classid: new_classid,
                        domain_id: new_dmid,
                        name: classname.clone(),
                        mindevcount: mindevcount,
                        replpolicy: replpolicy,
                    };
                    let qcl = qualified_class_name(new_dmid, &classname);
                    self.class_cache.borrow_mut().add(db_class, (dmid, classid), qcl);
                    class = self.class_cache.borrow().find_by_id(&(dmid, classid));
                }
            });

            class
        })
    }

    pub fn class_by_name(&self, domain_name: &str, class_name: &str) -> Option<Rc<Class>> {
        self.domain_by_name(domain_name).and_then(|domain| {
            let qcl = qualified_class_name(domain.dmid, class_name);
            let mut class = self.class_cache.borrow().find_by_name(&qcl);
            class.clone().or_else(|| {
                self.select(
                    "SELECT dmid, classid, classname, mindevcount, hashtype, replpolicy FROM class WHERE dmid = ? and classname = ?",
                    (domain.dmid, class_name), |result| {
                        if let Some(Ok(db_row)) = result.next() {
                            let (new_dmid, new_classid, new_class_name, mindevcount, _hashtype, replpolicy) =
                                value::from_row::<(u16, u8, String, u8, u8, String)>(db_row);
                            let db_class = Class {
                                classid: new_classid,
                                domain_id: new_dmid,
                                name: new_class_name.clone(),
                                mindevcount: mindevcount,
                                replpolicy: replpolicy,
                            };
                            let new_qcl = qualified_class_name(new_dmid, &new_class_name);
                            self.class_cache.borrow_mut().add(db_class, (new_dmid, new_classid), new_qcl);
                            class = self.class_cache.borrow().find_by_id(&(new_dmid, new_classid));
                        }
                    });

                class
            })

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

fn qualified_class_name(domain_id: u16, class_name: &str) -> String {
    format!("{}_{}", domain_id, class_name)
}

#[derive(Debug, Clone)]
pub struct Domain {
    pub dmid: u16,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub classid: u8,
    pub domain_id: u16,
    pub name: String,
    pub mindevcount: u8,
    // pub hashtype: u8,
    pub replpolicy: String,
}

#[derive(Debug, Clone)]
pub struct Fid {
    pub fid: u64,
    pub domain_id: u16,
    pub key: String,
    pub length: u64,
    pub class_id: u8,
    pub devcount: u8,
}

#[derive(Debug, Clone)]
struct ObjectCache<I: Eq + Hash + Debug, T> {
    by_id: HashMap<I, Rc<T>>,
    by_name: HashMap<String, Rc<T>>,
}

impl<I: Eq + Hash + Debug, T> ObjectCache<I, T> {
    fn new() -> ObjectCache<I, T> {
        ObjectCache {
            by_id: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    fn add(&mut self, object: T, id: I, name: String) {
        let object_rc = Rc::new(object);
        self.by_id.entry(id).or_insert(object_rc.clone());
        self.by_name.entry(name).or_insert(object_rc.clone());
    }

    fn find_by_id(&self, id: &I) -> Option<Rc<T>> {
        self.by_id.get(id).cloned()
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
