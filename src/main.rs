extern crate mysql;
extern crate url;

mod store;

use mysql::conn::MyOpts;
use mysql::conn::pool::MyPool;
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
        store: store::Store::new(pool),
    };
    handler.handle(stream);
}

struct Handler {
    store: store::Store,
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
                .and_then(|domain_name| self.store.get_domain_id(domain_name))
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

        let keys = self.store.get_matching_keys(domain_id, prefix, after, limit);
        let mut returned_values: Vec<(String, String)> = keys.iter().enumerate().map(|(i, k)| (format!("key_{}", i + 1), k.clone())).collect();
        returned_values.push(("key_count".to_string(), keys.len().to_string()));
        returned_values.push(("next_after".to_string(), keys.last().unwrap().clone()));

        Ok(format!("OK {}", form_urlencoded::serialize(returned_values)))
    }
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
