#![allow(dead_code, unused_variables)]

use mogilefs_common::{Response, MogError, MogResult};
use mogilefs_common::requests::*;
use rand;
use std::net::{SocketAddr, TcpStream};
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread::{self, JoinHandle};
use super::backend::TrackerBackend;

enum RequestInner {
    // Real(mogilefs_common::Request),
    Stop,
}

struct ProxyRequest {
    inner: RequestInner,
    respond: Sender<MogResult<Box<Response>>>,
}

struct ProxyResponse {
    inner: Response,
}

pub struct ProxyTrackerBackend {
    trackers: Vec<SocketAddr>,
    conn_thread_handle: Option<JoinHandle<()>>,
    conn_thread_sender: Option<Mutex<Sender<ProxyRequest>>>,
}

impl ProxyTrackerBackend {
    pub fn new(trackers: &[SocketAddr]) -> ProxyTrackerBackend {
        ProxyTrackerBackend {
            trackers: trackers.to_owned(),
            conn_thread_handle: None,
            conn_thread_sender: None,
        }
    }

    fn random_tracker_addr(&self) -> MogResult<SocketAddr> {
        let mut rng = rand::thread_rng();
        let mut sample = rand::sample(&mut rng, self.trackers.iter(), 1);
        sample.pop().cloned().ok_or(MogError::NoTrackers)
    }

    fn with_conn_thread_sender<F, T>(&self, callback: F) -> MogResult<T>
        where F: FnOnce(&Sender<ProxyRequest>) -> MogResult<T>
    {
        let sender_mutex = try!(self.conn_thread_sender.as_ref().ok_or(MogError::NoConnection));
        match sender_mutex.lock() {
            Ok(locked) => callback(&locked),
            Err(poisoned) => {
                let locked = poisoned.into_inner();
                callback(&locked)
            }
        }
    }

    fn conn_thread_sender_clone(&self) -> MogResult<Sender<ProxyRequest>> {
        self.with_conn_thread_sender(|locked| Ok(locked.clone()))
    }

    fn create_conn_thread(&mut self) -> MogResult<()> {
        let (tx, rx) = mpsc::channel::<ProxyRequest>();
        let tracker_addr = try!(self.random_tracker_addr());

        self.conn_thread_sender = Some(Mutex::new(tx));
        self.conn_thread_handle = Some(thread::spawn(move|| {
            connection_thread(tracker_addr, rx);
        }));

        Ok(())
    }

    fn send_request(&mut self, // req: mogilefs_common::Request
                    ) -> MogResult<Box<Response>> {
        if self.conn_thread_sender.is_none() {
            try!(self.create_conn_thread());
        }

        let (tx, rx) = mpsc::channel();
        let sender = try!(self.conn_thread_sender_clone());

        // try!(sender.send(Request { inner: RequestInner::Real(req), respond: tx }));
        rx.recv().map_err(|e| MogError::from(e))
    }
}

fn connection_thread(addr: SocketAddr, requests: Receiver<ProxyRequest>) {
    let conn = match TcpStream::connect(addr) {
        Ok(stream) => stream,
        Err(e) => {
            error!("Failed to connect to {:?}: {}", addr, e);
            return;
        }
    };

    for request in requests.iter() {
        match request.inner {
            RequestInner::Stop => {
                info!("Stopping and closing connection thread...");
                break;
            },
            // RequestInner::Real(inner) => {
            //     debug!("Sending request {:?} to {:?}...", inner, conn.peer_addr());
            // },
        }
    }
}

impl TrackerBackend for ProxyTrackerBackend {
    fn create_domain(&self, _req: &CreateDomain) -> MogResult<CreateDomain> {
        unimplemented!()
    }

    fn create_open(&self, _req: &CreateOpen) -> MogResult<CreateOpenResponse> {
        unimplemented!()
    }

    fn create_close(&self, _req: &CreateClose) -> MogResult<()> {
        unimplemented!()
    }

    fn get_paths(&self, _req: &GetPaths) -> MogResult<GetPathsResponse> {
        unimplemented!()
    }
    
    fn file_info(&self, _req: &FileInfo) -> MogResult<FileInfoResponse> {
        unimplemented!()
    }
    
    fn delete(&self, _req: &Delete) -> MogResult<()> {
        unimplemented!()
    }

    fn rename(&self, _req: &Rename) -> MogResult<()> {
        unimplemented!()
    }

    fn list_keys(&self, req: &ListKeys) -> MogResult<ListKeysResponse> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use mogilefs_common::MogError;
    use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
    use std::sync::mpsc::{self, Sender};
    use std::thread::{self, JoinHandle};
    use super::*;
    use super::{connection_thread, ProxyRequest, RequestInner};

    fn tracker_addr_list() -> Vec<SocketAddr> {
        let rv: Vec<SocketAddr> = vec![ "127.0.0.1:7101", "127.0.0.1:7102", "[::1]:7103" ]
            .iter()
            .flat_map(|a| a.to_socket_addrs().unwrap())
            .collect();
        assert_eq!(3, rv.len());
        rv
    }

    fn stop_conn_thread(handle: JoinHandle<()>, sender: Sender<ProxyRequest>) {
        let (res_tx, _) = mpsc::channel();
        sender.send(ProxyRequest { inner: RequestInner::Stop, respond: res_tx }).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn initial_state() {
        let addr_list = tracker_addr_list();
        let backend = ProxyTrackerBackend::new(&addr_list);
        assert_eq!(addr_list, backend.trackers);
        assert!(matches!(backend.conn_thread_handle, None));
        assert!(matches!(backend.conn_thread_sender, None));
        assert!(matches!(backend.conn_thread_sender_clone(), Err(MogError::NoConnection)));
        assert!(matches!(backend.with_conn_thread_sender(move|_| Ok(())), Err(MogError::NoConnection)))
    }

    #[test]
    fn no_trackers() {
        let backend = ProxyTrackerBackend::new(&[]);
        assert!(matches!(backend.random_tracker_addr(), Err(MogError::NoTrackers)));
    }

    #[test]
    fn random_tracker_addr() {
        let addr_list = tracker_addr_list();
        let backend = ProxyTrackerBackend::new(&addr_list);
        let mut seen: Vec<bool> = addr_list.iter().map(|_| false).collect();
        let (mut loops, max_loops) = (0, addr_list.len() * 100);

        loop {
            loops += 1;
            let addr = backend.random_tracker_addr().unwrap();
            let index = addr_list.iter().position(|&a| a == addr);
            assert!(index.is_some());
            seen[index.unwrap()] = true;
            if seen.iter().all(|&s| s) || loops >= max_loops {
                break;
            }
        }

        debug!("Seen all addrs after {} loops.", loops);
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn can_stop_connection_thread() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (req_tx, req_rx) = mpsc::channel();

        // Gosh, it would be great if I could time this out somehow by using Stable Rust...
        let conn_thread = thread::spawn(move|| connection_thread(listener.local_addr().unwrap().clone(), req_rx));
        stop_conn_thread(conn_thread, req_tx);
    }

    #[test]
    fn creates_conn_thread() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut backend = ProxyTrackerBackend::new(&[ listener.local_addr().unwrap() ]);
        assert!(backend.create_conn_thread().is_ok());

        assert!(backend.conn_thread_handle.is_some());
        assert!(backend.conn_thread_sender.is_some());
        assert!(matches!(backend.with_conn_thread_sender(|_| Ok(())), Ok(())));
        assert!(matches!(backend.conn_thread_sender_clone(), Ok(..)));

        stop_conn_thread(
            backend.conn_thread_handle.take().unwrap(),
            backend.conn_thread_sender_clone().unwrap());
    }
}
