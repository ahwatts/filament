use std::io::{self, Write, BufRead, BufReader};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::thread;
use super::Tracker;
use mogilefs_common::{Backend, Renderable};

pub struct ThreadedListener<B: Backend> {
    listener: TcpListener,
    tracker: Arc<Tracker<B>>,
}

impl<B: 'static + Backend> ThreadedListener<B> {
    pub fn new<S: ToSocketAddrs>(addr: S, tracker: Tracker<B>) -> Result<ThreadedListener<B>, io::Error> {
        Ok(ThreadedListener {
            listener: try!(TcpListener::bind(addr)),
            tracker: Arc::new(tracker),
        })
    }

    pub fn run(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let conn_tracker = self.tracker.clone();

                    thread::spawn(move|| {
                        let peer_addr = stream.peer_addr();
                        info!("New connection from {:?}", peer_addr);
                        match handle_connection(stream, conn_tracker) {
                            Ok(_) => {},
                            Err(e) => {
                                error!("Error handling connection from {:?}: {}", peer_addr, e);
                            }
                        }
                        info!("Shutting down connection from {:?}", peer_addr);
                    });
                },
                Err(e) => {
                    error!("Connection failed: {}", e);
                }
            }
        }
    }
}

fn handle_connection<B: Backend>(mut writer: TcpStream, tracker: Arc<Tracker<B>>) -> Result<(), io::Error> {
    let reader = BufReader::new(try!(writer.try_clone()));

    for line in reader.split(b'\n') {
        let mut line = try!(line);
        debug!("request line = {:?}", String::from_utf8_lossy(&line));
        if line.last() == Some(&b'\r') { line.pop(); }
        let response = tracker.handle_bytes(line.as_ref());

        // Despite both arms being identical, I have to break it out
        // because the result itself is not Renderable.
        let rendered = match response {
            Ok(resp) => resp.render(),
            Err(e) => e.render(),
        };

        debug!("response line = {:?}", rendered);
        try!(write!(writer, "{}\r\n", rendered));
    }

    Ok(())
}
