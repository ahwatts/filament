extern crate argparse;
extern crate mysql;
extern crate url;

mod fid;
mod store;

use argparse::ArgumentParser;
use mysql::conn::MyOpts;
use mysql::conn::pool::MyPool;
use std::collections::HashMap;
use std::default::Default;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use url::form_urlencoded;
use self::store::Store;
use self::fid::Fid;

type CommandArgs = HashMap<String, Vec<String>>;
type TrackerResult = Result<String, String>;

fn main() {
    let mut opts: Options = Default::default();
    opts = opts.parse_args();
    let listen_addr: &str = &opts.listen;
    let listener = TcpListener::bind(listen_addr).unwrap();

    let mysql_opts = MyOpts {
        tcp_addr: Some(opts.mysql_host),
        tcp_port: opts.mysql_port as u16,
        user: Some(opts.mysql_user),
        pass: Some(opts.mysql_pass),
        db_name: Some(opts.mysql_db),
        ..Default::default()
    };

    let db_pool = MyPool::new(mysql_opts).unwrap_or_else(|e| {
        panic!("Error connecting to MySQL: {}", e);
    });

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

#[derive(Debug)]
struct Options {
    listen: String,
    mysql_host: String,
    mysql_port: i32,
    mysql_user: String,
    mysql_pass: String,
    mysql_db: String,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            listen: "0.0.0.0:7001".to_string(),
            mysql_host: "127.0.0.1".to_string(),
            mysql_port: 3306,
            mysql_user: "mogile".to_string(),
            mysql_pass: "".to_string(),
            mysql_db: "mogilefs".to_string(),
        }
    }
}

impl Options {
    fn parse_args(mut self) -> Options {
        {
            let mut parser = ArgumentParser::new();
            parser.set_description("A partial clone for the MogileFS tracker daemon.");
            parser.refer(&mut self.listen)    .add_option(&[ "-l", "--listen"   ], argparse::Store, "The host:port for the tracker to listen on.");
            parser.refer(&mut self.mysql_host).add_option(&[ "-H", "--db-host"  ], argparse::Store, "The MySQL server address.");
            parser.refer(&mut self.mysql_port).add_option(&[ "-P", "--db-port"  ], argparse::Store, "The MySQL server port.");
            parser.refer(&mut self.mysql_user).add_option(&[ "-u", "--user"     ], argparse::Store, "The username for the MySQL connection.");
            parser.refer(&mut self.mysql_pass).add_option(&[ "-p", "--password" ], argparse::Store, "The password for the MySQL connection.");
            parser.refer(&mut self.mysql_db)  .add_option(&[ "-D", "--db-name"  ], argparse::Store, "The MySQL database name.");
            parser.parse_args_or_exit();
        }

        self
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
            Some("get_paths") => self.get_paths_command(&parse_query_string(args.unwrap().as_bytes())),
            _ => Err("because f*** you, that's why.".to_string()),
        }
    }

    fn list_keys_command(&self, args: &CommandArgs) -> TrackerResult {
        // println!("args = {:?}", args);

        let domain_id = try!(get_domain_id_from_args(args, &self.store));
        let prefix = args.get("prefix").and_then(|v| v.first());
        let after = args.get("after").and_then(|v| v.first());
        let limit = args.get("limit").and_then(|v| v.first())
            .and_then(|s| i32::from_str_radix(s, 10).ok())
            .and_then(|d| clamp(d, 0, 1000))
            .unwrap_or(1000);

        // println!("domain_id = {:?} prefix = {:?} after = {:?} limit = {:?}", domain_id, prefix, after, limit);

        if prefix.is_some() && after.is_some() {
            let real_after: &str = after.unwrap();
            let real_prefix: &str = prefix.unwrap();
            if !real_prefix.starts_with(real_after) {
                return Err("ERR after_mismatch Pattern+does+not+match+the+after-value%3F".to_string());
            }
        }

        let keys = try!(self.store.get_matching_keys(domain_id, prefix, after, limit).map_err(|e| format!("ERR {}", e.to_error_string())));
        let mut returned_values: Vec<(String, String)> = keys.iter().enumerate().map(|(i, k)| (format!("key_{}", i + 1), k.clone())).collect();
        returned_values.push(("key_count".to_string(), keys.len().to_string()));
        returned_values.push(("next_after".to_string(), keys.last().unwrap().clone()));

        Ok(format!("OK {}", form_urlencoded::serialize(returned_values)))
    }

    fn get_paths_command(&self, args: &CommandArgs) -> TrackerResult {
        println!("args = {:?}", args);

        let domain_id = try!(get_domain_id_from_args(args, &self.store));
        let key = try!(args.get("key").and_then(|v| v.first()).ok_or("ERR no_key No+key+provided"));
        // let path_count = args.get("pathcount").and_then(|v| v.first())
        //     .and_then(|s| i32::from_str_radix(s, 10).ok())
        //     .and_then(|d| clamp(d, 2, 10))
        //     .unwrap_or(2);

        let fid = try!(Fid::new_from_dmid_and_key(&self.store, domain_id, key).map_err(|e| format!("ERR {}", e)));

        let mut returned_values: Vec<(String, String)> = vec![];
        returned_values.push(("paths".to_string(), fid.device_ids.len().to_string()));
        for (i, device_id) in fid.device_ids.iter().enumerate() {
            returned_values.push((format!("path{}", (i + 1)), device_id.to_string()));
        }
        Ok(format!("OK {}", form_urlencoded::serialize(returned_values)))
    }
}

fn get_domain_id_from_args(args: &CommandArgs, store: &store::Store) -> Result<i32, String> {
    let domain_name   = try!(args.get("domain").and_then(|v| v.first()).ok_or("ERR no_domain No+domain+provided"));
    let domain_id_opt = try!(store.get_domain_id(domain_name).map_err(|e| format!("ERR {}", e.to_error_string())));
    domain_id_opt.ok_or("ERR unreg_domain Domain+name+invalid/not+found".to_string())
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
