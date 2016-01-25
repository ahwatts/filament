//! Utilities for working with the MogileFS database.

use chrono::UTC;
use mogilefs_common::{MogError, MogResult};
use mysql::conn::{MyOpts, QueryResult};
use mysql::conn::pool::MyPool;
use mysql::error::MyError;
use mysql::value::{self, ToRow};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

type DomainCache = ObjectCache<u16, Domain>;
type ClassCache = ObjectCache<(u16, u8), Class>;

enum DomainSearch<'a> { Name(&'a str), Id(u16) }
enum ClassSearch<'a> { Name(u16, &'a str), Id(u16, u8) }

pub struct DataStore {
    pool: MyPool,
    domain_cache: RwLock<DomainCache>,
    class_cache: RwLock<ClassCache>,
}

impl DataStore {
    pub fn new(opts: MyOpts) -> MogResult<DataStore> {
        let pool = match MyPool::new(opts) {
            Ok(p) => p,
            Err(e) => return Err(MogDbError::from(e).into()),
        };

        Ok(DataStore {
            pool: pool,
            domain_cache: RwLock::new(ObjectCache::new()),
            class_cache: RwLock::new(ObjectCache::new()),
        })
    }

    pub fn new_from_pool(pool: MyPool) -> MogResult<DataStore> {
        Ok(DataStore {
            pool: pool,
            domain_cache: RwLock::new(ObjectCache::new()),
            class_cache: RwLock::new(ObjectCache::new()),
        })
    }

    pub fn domain_by_id(&self, dmid: u16) -> Option<Arc<Domain>> {
        let search = DomainSearch::Id(dmid);
        self.domain_from_cache(&search).or_else(|| {
            self.find_and_cache_domain("SELECT dmid, namespace FROM domain WHERE dmid = ?", (dmid,));
            self.domain_from_cache(&search)
        })
    }

    pub fn domain_by_name(&self, name: &str) -> Option<Arc<Domain>> {
        let search = DomainSearch::Name(name);
        self.domain_from_cache(&search).or_else(|| {
            self.find_and_cache_domain("SELECT dmid, namespace FROM domain WHERE namespace = ?", (name,));
            self.domain_from_cache(&search)
        })
    }

    fn find_and_cache_domain<Q: AsRef<str>, P: ToRow>(&self, query: Q, params: P) {
        if let Some(db_domain) = self.domain_from_db(query.as_ref(), params) {
            let (id, name) = (db_domain.dmid, db_domain.name.clone());
            let mut cache = match self.domain_cache.write() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            cache.add(db_domain, id, name);
        }
    }

    fn domain_from_cache(&self, search: &DomainSearch) -> Option<Arc<Domain>> {
        use self::DomainSearch::*;

        let cache = match self.domain_cache.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };

        match search {
            &Name(domain_name) => cache.find_by_name(domain_name),
            &Id(ref domain_id) => cache.find_by_id(domain_id),
        }
    }

    fn domain_from_db<Q: AsRef<str>, P: ToRow>(&self, query: Q, params: P) -> Option<Domain> {
        let dm_rslt = self.select(query.as_ref(), params, |result| {
            match result.next() {
                Some(Ok(db_row)) => {
                    let (dmid, name) = value::from_row::<(u16, String)>(db_row);
                    Ok(Some(Domain { dmid: dmid, name: name }))
                },
                Some(Err(e)) => Err(MogDbError::from(e)),
                None => Ok(None)
            }
        });

        match dm_rslt {
            Ok(d) => d,
            Err(e) => {
                error!("Error getting domain from database: {:?}", e);
                None
            },
        }
    }

    pub fn class_by_id(&self, dmid: u16, classid: u8) -> Option<Arc<Class>> {
        let search = ClassSearch::Id(dmid, classid);
        self.class_from_cache(&search).or_else(|| {
            self.find_and_cache_class(
                "SELECT dmid, classid, classname, mindevcount, hashtype, replpolicy FROM class WHERE dmid = ? and classid = ?",
                (dmid, classid));
            self.class_from_cache(&search)
        })
    }

    pub fn class_by_name(&self, domain_name: &str, class_name: &str) -> Option<Arc<Class>> {
        self.domain_by_name(domain_name).and_then(|domain| {
            let search = ClassSearch::Name(domain.dmid, class_name);
            self.class_from_cache(&search).or_else(|| {
                self.find_and_cache_class(
                    "SELECT dmid, classid, classname, mindevcount, hashtype, replpolicy FROM class WHERE dmid = ? and classname = ?",
                    (domain.dmid, class_name));
                self.class_from_cache(&search)
            })
        })
    }

    fn find_and_cache_class<Q: AsRef<str>, P: ToRow>(&self, query: Q, params: P) {
        if let Some(class) = self.class_from_db(query.as_ref(), params) {
            let (classid, dmid, class_name) = (class.classid, class.domain_id, class.name.clone());
            let mut cache = match self.class_cache.write() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            cache.add(class, (dmid, classid), qualified_class_name(dmid, &class_name));
        }
    }

    fn class_from_cache(&self, search: &ClassSearch) -> Option<Arc<Class>> {
        use self::ClassSearch::*;

        let cache = match self.class_cache.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };

        match search {
            &Name(domain_id, class_name) => cache.find_by_name(&qualified_class_name(domain_id, class_name)),
            &Id(domain_id, class_id) => cache.find_by_id(&(domain_id, class_id)),
        }
    }

    fn class_from_db<Q: AsRef<str>, P: ToRow>(&self, query: Q, params: P) -> Option<Class> {
        let cls_rslt = self.select(query.as_ref(), params, |result| {
            match result.next() {
                Some(Ok(db_row)) => {
                    let (dmid, classid, classname, mindevcount, _hashtype, replpolicy) =
                        value::from_row::<(u16, u8, String, u8, Option<u8>, Option<String>)>(db_row);
                    Ok(Some(Class {
                        classid: classid,
                        domain_id: dmid,
                        name: classname,
                        mindevcount: mindevcount,
                        replpolicy: replpolicy,
                    }))
                },
                Some(Err(e)) => Err(MogDbError::from(e)),
                None => Ok(None)
            }
        });

        match cls_rslt {
            Ok(c) => c,
            Err(e) => {
                error!("Error getting class from database: {:?}", e);
                None
            },
        }
    }

    pub fn fid_by_key(&self, domain_name: &str, key: &str) -> Option<Fid> {
        self.domain_by_name(domain_name).and_then(|domain| {
            let fid_rslt = self.select(
                "SELECT fid, dmid, dkey, length, classid, devcount FROM file WHERE dmid = ? AND dkey = ?",
                (domain.dmid, key), |result| {
                    match result.next() {
                        Some(Ok(db_row)) => {
                            let (fid, new_dmid, new_key, length, classid, devcount) =
                                value::from_row::<(u64, u16, String, u64, u8, u8)>(db_row);
                            Ok(Some(Fid {
                                fid: fid,
                                domain_id: new_dmid,
                                key: new_key,
                                length: length,
                                class_id: classid,
                                devcount: devcount,
                            }))
                        },
                        Some(Err(e)) => Err(MogDbError::from(e)),
                        None => Ok(None),
                    }
                });

            match fid_rslt {
                Ok(f) => f,
                Err(e) => {
                    error!("Error getting fid from database: {:?}", e);
                    None
                },
            }
        })
    }

    fn select<Q: AsRef<str>, P: ToRow, F, R>(&self, query: Q, params: P, callback: F) -> MogDbResult<R>
        where F: FnOnce(&mut QueryResult) -> MogDbResult<R>
    {
        let mut conn = try!(self.pool.get_conn());
        trace!("Executing query: {:?}", query.as_ref());
        let start = UTC::now();
        let result = conn.prep_exec(query.as_ref(), params);
        let end = UTC::now();
        trace!("Query took {:?}", end - start);

        match result {
            Ok(mut result_set) => {
                debug!("Select query ({:?}): {:?}", end - start, query.as_ref());
                callback(&mut result_set)
            },
            Err(e) => {
                error!("Error with query {:?}: {}", query.as_ref(), e);
                Err(MogDbError::from(e))
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
    pub replpolicy: Option<String>,
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
    by_id: HashMap<I, Arc<T>>,
    by_name: HashMap<String, Arc<T>>,
}

impl<I: Eq + Hash + Debug, T> ObjectCache<I, T> {
    fn new() -> ObjectCache<I, T> {
        ObjectCache {
            by_id: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    fn add(&mut self, object: T, id: I, name: String) {
        let object_rc = Arc::new(object);
        self.by_id.entry(id).or_insert(object_rc.clone());
        self.by_name.entry(name).or_insert(object_rc.clone());
    }

    fn find_by_id(&self, id: &I) -> Option<Arc<T>> {
        self.by_id.get(id).cloned()
    }

    fn find_by_name(&self, name: &str) -> Option<Arc<T>> {
        self.by_name.get(name).cloned()
    }
}

#[derive(Debug)]
enum MogDbError {
    Common(MogError),
    Database(MyError),
}

impl From<MogError> for MogDbError {
    fn from(err: MogError) -> MogDbError {
        MogDbError::Common(err)
    }
}

impl From<MyError> for MogDbError {
    fn from(err: MyError) -> MogDbError {
        MogDbError::Database(err)
    }
}

impl Into<MogError> for MogDbError {
    fn into(self) -> MogError {
        match self {
            MogDbError::Common(e) => e,
            MogDbError::Database(e) => MogError::Database(format!("{}", e)),
        }
    }
}

type MogDbResult<T> = Result<T, MogDbError>;

#[cfg(test)]
mod tests {
    use mogilefs_common::MogResult;
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
        static ref FILAMENT_TEST_DOMAIN_NAME: String = env::var("FILAMENT_TEST_DOMAIN").ok().unwrap_or("test_domain".to_string());
        static ref FILAMENT_TEST_CLASS_ID: u8 = u8::from_str(&env::var("FILAMENT_TEST_CLASS_ID").ok().unwrap_or("1".to_string())).unwrap();
        static ref FILAMENT_TEST_CLASS_NAME: String = env::var("FILAMENT_TEST_CLASS").ok().unwrap_or("test_class".to_string());
        static ref FILAMENT_TEST_KEY: String = env::var("FILAMENT_TEST_KEY").ok().unwrap_or("test/key/1".to_string());
    }

    fn data_store_fixture() -> MogResult<DataStore> {
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

    #[test]
    fn test_get_class_by_id() {
        let store = test_store!();
        let class = store.class_by_id(*FILAMENT_TEST_DOMAIN_ID, *FILAMENT_TEST_CLASS_ID).unwrap();
        assert_eq!(*FILAMENT_TEST_CLASS_ID, class.classid);
        assert_eq!(*FILAMENT_TEST_CLASS_NAME, class.name);
        assert_eq!(*FILAMENT_TEST_DOMAIN_ID, class.domain_id);
    }

    #[test]
    fn test_get_class_by_name() {
        let store = test_store!();
        let class = store.class_by_name(&*FILAMENT_TEST_DOMAIN_NAME, &*FILAMENT_TEST_CLASS_NAME).unwrap();
        assert_eq!(*FILAMENT_TEST_CLASS_ID, class.classid);
        assert_eq!(*FILAMENT_TEST_CLASS_NAME, class.name);
        assert_eq!(*FILAMENT_TEST_DOMAIN_ID, class.domain_id);
    }

    #[test]
    fn test_get_fid() {
        let store = test_store!();
        let fid = store.fid_by_key(&*FILAMENT_TEST_DOMAIN_NAME, &*FILAMENT_TEST_KEY);
        assert!(fid.is_some());

        let fid2 = fid.unwrap();
        assert_eq!(*FILAMENT_TEST_KEY, fid2.key);
    }
}
