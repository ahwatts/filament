pub mod tracker {
    use std::error;
    use std::fmt::{self, Display, Formatter};
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpStream};
    use std::result;

    #[derive(Debug)]
    pub enum Error {
        Other(String),
    }

    impl Display for Error {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            match *self {
                Error::Other(ref s) => write!(f, "{}", s),
            }
        }
    }

    impl error::Error for Error {
        fn description(&self) -> &str {
            match *self {
                Error::Other(ref s) => s,
            }
        }
    }

    pub type Result<'a, T> = result::Result<T, Error>;

    #[derive(Clone)]
    pub struct Handler;

    impl Handler {
        pub fn new() -> Handler {
            Handler
        }
        
        pub fn handle(&self, mut stream: TcpStream) {
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

        fn dispatch_command(&self, line: &str) -> Result<String> {
            let mut toks = line.split(" ");
            let command = toks.next();

            match command {
                _ => Err(Error::Other("because f*** you, that's why.".to_string())),
            }
        }
    }
}
