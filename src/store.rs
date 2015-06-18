use mysql::conn::pool::MyPool;
use mysql::value::{Value, ToValue, FromValue};
use std::collections::HashMap;
use url::percent_encoding;

pub enum StoreError {
    Db(String),
}

impl StoreError {
    fn new_db_error<T: AsRef<str>>(msg: T) -> StoreError {
        StoreError::Db(msg.as_ref().to_string())
    }

    pub fn to_error_string(&self) -> String {
        let err_type = match self {
            &StoreError::Db(_) => "db_error",
        };

        let err_msg = match self {
            &StoreError::Db(ref err_str) => {
                percent_encoding::percent_encode(err_str.as_bytes(), percent_encoding::FORM_URLENCODED_ENCODE_SET)
            }
        };

        format!("{} {}", err_type, err_msg)
    }
}

type StoreResult<T> = Result<T, StoreError>;

pub struct Store {
    pool: MyPool,
}

impl Store {
    pub fn new(pool: MyPool) -> Store {
        Store {
            pool: pool,
        }
    }

    pub fn get_domain_id<T: AsRef<str>>(&self, domain_name: T) -> StoreResult<Option<i32>> {
        let domains = run_query(&self.pool, "SELECT dmid FROM domain WHERE namespace = ?", &[ &domain_name.as_ref() ]);

        match domains {
            Err(e) => {
                Err(StoreError::new_db_error(format!("Error querying domains: {:?}", e)))
            },
            Ok(rows) => {
                match rows.first() {
                    None => Ok(None),
                    Some(row) => {
                        match row.get("dmid") {
                            Some(&Value::Int(v)) => Ok(Some(v as i32)),
                            _ => Ok(None),
                        }
                    }
                }
            }
        }
    }

    pub fn get_matching_keys<T, U>(&self, domain_id: i32, prefix: Option<T>, after: Option<U>, limit: i32) -> StoreResult<Vec<String>>
        where T: AsRef<str>, U: AsRef<str>
    {
        let mut prefix_param = prefix.map(|n| n.as_ref().to_string()).unwrap_or("".to_string());
        let after_param = after.map(|n| n.as_ref().to_string()).unwrap_or("".to_string());

        prefix_param = prefix_param
            .replace("\\", "\\\\")
            .replace("%", "\\%")
            .replace("_", "\\_");
        prefix_param.push_str("%");

        println!("dmid = {:?} prefix_param = {:?} after_param = {:?} limit = {:?}",
                 domain_id, prefix_param, after_param, limit);

        let keys = run_query(
            &self.pool,
            "SELECT dkey FROM file WHERE dmid = ? AND dkey LIKE ? ESCAPE ? AND dkey > ? ORDER BY dkey LIMIT ?",
            &[ &domain_id, &prefix_param, &"\\", &after_param, &limit]);

        let mut rv = vec![];

        match keys {
            Err(e) => {
                return Err(StoreError::new_db_error(format!("Error querying matching keys: {:?}", e)));
            },
            Ok(rows) => {
                for row in rows {
                    match row.get("dkey") {
                        Some(&Value::Bytes(ref bs)) => {
                            rv.push(String::from_utf8_lossy(bs).into_owned());
                        },
                        _ => {},
                    }
                }
            }
        }

        Ok(rv)
    }

    pub fn get_file_row_from_dmid_and_key<T: AsRef<str>>(&self, domain_id: i32, key: T) -> StoreResult<Option<HashMap<String, Value>>> {
        let result = run_query(
            &self.pool,
            "SELECT fid, dmid, dkey, length, classid, devcount FROM file WHERE dmid = ? AND dkey = ? LIMIT 1",
            &[ &domain_id, &key.as_ref() ]);

        match result {
            Err(e) => Err(StoreError::new_db_error(format!("Error querying file row: {:?}", e))),
            Ok(rows) => Ok(rows.into_iter().next()),
        }
    }

    pub fn get_devids_for_fid(&self, fid: i32) -> StoreResult<Vec<i32>> {
        let result = run_query(&self.pool, "SELECT devid FROM file_on WHERE fid = ?", &[ &fid ]);

        match result {
            Err(e) => Err(StoreError::new_db_error(format!("Error querying file_on rows: {:?}", e))),
            Ok(rows) => Ok(rows.into_iter().map(|r| i32::from_value(&r["devid"])).collect()),
        }
    }
}

fn run_query(pool: &MyPool, query: &str, args: &[&ToValue]) -> Result<Vec<HashMap<String, Value>>, String> {
    let mut statement = try!(pool.prepare(query).map_err(|e| format!("MySQL error preparing statement ({:?}): {}", query, e)));
    let columns: HashMap<usize, String> = {
        let mut v = HashMap::new();
        match statement.columns_ref() {
            Some(columns) => {
                for (i, col) in columns.iter().enumerate() {
                    v.insert(i, String::from_utf8_lossy(&col.name).into_owned());
                }
            },
            None => {
                println!("No columns in statement ({:?})", query);
            },
        }
        v
    };

    let result = try!(statement.execute(args).map_err(|e| format!("MySQL error executing statement ({:?}): {}", query, e)));
    let mut result_set = vec![];

    for row_result in result {
        match row_result {
            Ok(row) => {
                let mut row_hash = HashMap::new();
                for (i, value) in row.iter().enumerate() {
                    match columns.get(&i) {
                        Some(column_name) => { row_hash.insert(column_name.clone(), value.clone()); },
                        None => { println!("Empty column for {:?}", i); },
                    }
                }
                result_set.push(row_hash);
            },
            Err(e) => {
                return Err(format!("MySQL error processing results for statement ({:?}): {}", query, e));
            },
        }
    }

    Ok(result_set)
}
