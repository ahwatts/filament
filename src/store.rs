use mysql::conn::pool::MyPool;
use mysql::value::{ToValue, Value};
use std::collections::HashMap;

pub struct Store {
    pool: MyPool,
}

impl Store {
    pub fn new(pool: MyPool) -> Store {
        Store {
            pool: pool,
        }
    }

    pub fn get_domain_id(&self, domain_name: &str) -> Option<i32> {
        let domains = run_query(&self.pool, "SELECT dmid FROM domain WHERE namespace = ?", &[ &domain_name ]);

        match domains {
            Err(e) => {
                println!("Error querying domains: {:?}", e);
                None
            },
            Ok(rows) => {
                match rows.first() {
                    None => None,
                    Some(row) => {
                        match row.get("dmid") {
                            Some(&Value::Int(v)) => Some(v as i32),
                            _ => None,
                        }
                    }
                }
            }
        }
    }

    pub fn get_matching_keys(&self, domain_id: i32, prefix: Option<&String>, after: Option<&String>, limit: i32) -> Vec<String> {
        let mut prefix_param = prefix.cloned().unwrap_or("".to_string());
        let after_param = after.map(|n| n.as_ref()).unwrap_or("");

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
                println!("Error querying matching keys: {:?}", e);
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

        rv
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
