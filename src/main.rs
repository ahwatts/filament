extern crate mysql;
extern crate url;

use mysql::conn::MyOpts;
use mysql::conn::pool::MyPool;
use mysql::value::{ToValue, Value};
use std::collections::HashMap;
use std::default::Default;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use url::form_urlencoded;

type CommandArgs = HashMap<String, Vec<String>>;
type TrackerResult = Result<String, String>;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7002").unwrap();
    let mysql_opts = MyOpts {
        user: Some("mogile username".to_string()),
        pass: Some("mogile password".to_string()),
        db_name: Some("mogilefs".to_string()),
        ..Default::default()
    };
    let db_pool = MyPool::new(mysql_opts).unwrap();

    for stream_result in listener.incoming() {
        match stream_result {
            Ok(stream) => {
                let pool = db_pool.clone();
                thread::spawn(move|| handle_client(stream, pool));
            },
            Err(e) => {
                panic!("Connection failed: {:?}", e);
            },
        }
    }
}

fn handle_client(stream: TcpStream, pool: MyPool) {
    let mut handler = Handler {
        pool: pool,
    };
    handler.handle(stream);
}

struct Handler {
    pool: MyPool,
}

impl Handler {
    fn handle(&mut self, mut stream: TcpStream) {
        println!("Connection received: local = {:?} remote = {:?}",
                 stream.local_addr(), stream.peer_addr());
        let reader = BufReader::new(stream.try_clone().unwrap());

        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    println!("request  = {:?}", line);
                    let response = self.dispatch_command(&line.trim_right());
                    println!("response = {:?}", response);

                    // Okay, both arms here are the same, but maybe they
                    // won't be in the future?
                    match response {
                        Ok(response_str) => {
                            write!(stream, "{}\r\n", response_str)
                                .unwrap_or_else(|e| println!("Error writing successful response: {:?}", e));
                        },
                        Err(err_str) => {
                            write!(stream, "{}\r\n", err_str)
                                .unwrap_or_else(|e| println!("Error writing error response: {:?}", e));
                        }
                    }
                },
                Err(e) => {
                    println!("Error with connection: {:?}", e);
                    break;
                }
            }
        }
    }

    fn dispatch_command(&mut self, line: &str) -> TrackerResult {
        let mut toks = line.split(" ");
        let command = toks.next();
        let args = toks.next();

        match command {
            Some("list_keys") => self.list_keys_command(&parse_query_string(args.unwrap().as_bytes())),
            _ => {
                Err("because f*** you, that's why.".to_string())
            }
        }
    }

    fn list_keys_command(&self, args: &CommandArgs) -> TrackerResult {
        println!("args = {:?}", args);

        let domain_id = try!{
            args.get("domain")
                .and_then(|v| v.first())
                .and_then(|domain_name| self.get_domain_id(domain_name))
                .ok_or("ERR no_domain No+domain+provided")
        };

        let prefix = args.get("prefix").and_then(|v| v.first());
        let after = args.get("after").and_then(|v| v.first());
        let limit = args.get("limit").and_then(|v| v.first())
            .and_then(|s| i32::from_str_radix(s, 10).ok())
            .and_then(|d| clamp(d, 0, 1000))
            .unwrap_or(1000);

        println!("domain_id = {:?} prefix = {:?} after = {:?} limit = {:?}", domain_id, prefix, after, limit);

        if prefix.is_some() && after.is_some() {
            let real_after: &str = after.unwrap();
            let real_prefix: &str = prefix.unwrap();
            if !real_prefix.starts_with(real_after) {
                return Err("ERR after_mismatch Pattern+does+not+match+the+after-value%3F".to_string());
            }
        }

        let keys = self.get_matching_keys(domain_id, prefix, after, limit);
        let mut returned_values: Vec<(String, String)> = keys.iter().enumerate().map(|(i, k)| (format!("key_{}", i + 1), k.clone())).collect();
        returned_values.push(("key_count".to_string(), keys.len().to_string()));
        returned_values.push(("next_after".to_string(), keys.last().unwrap().clone()));

        Ok(format!("OK {}", form_urlencoded::serialize(returned_values)))
    }

    fn get_domain_id(&self, domain_name: &str) -> Option<i32> {
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

    fn get_matching_keys(&self, domain_id: i32, prefix: Option<&String>, after: Option<&String>, limit: i32) -> Vec<String> {
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

fn parse_query_string(query_string: &[u8]) -> CommandArgs {
    let parsed = form_urlencoded::parse(query_string);
    parsed.into_iter().fold(HashMap::new(), |mut m, (k, v)| {
        m.entry(k).or_insert(vec![]).push(v); m
    })
}

fn clamp<T: Ord>(v: T, min: T, max: T) -> Option<T> {
    if v >= min && v <= max {
        Some(v)
    } else {
        None
    }
}
