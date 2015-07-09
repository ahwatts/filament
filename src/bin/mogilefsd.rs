extern crate argparse;

use argparse::ArgumentParser;
use std::collections::HashMap;
use std::default::Default;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, Ipv4Addr};
use std::thread;

type CommandArgs = HashMap<String, Vec<String>>;
type TrackerResult = Result<String, String>;

fn main() {
    let mut opts: Options = Default::default();
    opts.parser().parse_args_or_exit();
    let listener = TcpListener::bind((opts.listen_ip, opts.listen_port)).unwrap();

    for stream_result in listener.incoming() {
        match stream_result {
            Ok(stream) => {
                thread::spawn(move|| handle_client(stream));
            },
            Err(e) => {
                panic!("Connection failed: {:?}", e);
            },
        }
    }
}

#[derive(Debug)]
struct Options {
    listen_ip: Ipv4Addr,
    listen_port: u16,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            listen_ip: Ipv4Addr::new(0, 0, 0, 0),
            listen_port: 7002,
        }
    }
}

impl Options {
    fn parser(&mut self) -> ArgumentParser {
        let mut parser = ArgumentParser::new();
        parser.set_description("A partial clone for the MogileFS tracker daemon.");

        parser.refer(&mut self.listen_ip).add_option(
            &[ "-l", "--listen-ip" ],
            argparse::Store,
            "The host IP for the tracker to listen on.");

        parser.refer(&mut self.listen_port).add_option(
            &[ "-l", "--listen-ip" ],
            argparse::Store,
            "The port for the tracker to listen on.");

        parser
    }
}

fn handle_client(stream: TcpStream) {
    let mut handler = Handler;
    handler.handle(stream);
}

struct Handler;

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

        match command {
            _ => Err("because f*** you, that's why.".to_string()),
        }
    }
}
