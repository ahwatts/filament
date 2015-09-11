use std::cell::RefCell;
use mogilefs_client::MogClient;
use mogilefs_common::{Request, Response, MogError, MogResult, ToUrlencodedString, FromBytes};
use mogilefs_common::requests::*;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread::{self, JoinHandle};
use super::backend::TrackerBackend;

thread_local!{
    static SENDER: RefCell<Option<Sender<ProxyRequest>>> = RefCell::new(None)
}

enum RequestInner {
    Real(Box<Request>),
    Stop,
}

struct ProxyRequest {
    inner: RequestInner,
    respond: Sender<ProxyResponse>,
}

struct ProxyResponse {
    inner: MogResult<Box<Response>>,
}

pub struct ProxyTrackerBackend {
    trackers: Vec<SocketAddr>,
    conn_thread_handle: Option<JoinHandle<()>>,
    conn_thread_sender: Option<Mutex<Sender<ProxyRequest>>>,
}

impl ProxyTrackerBackend {
    pub fn new(trackers: &[SocketAddr]) -> MogResult<ProxyTrackerBackend> {
        let mut backend = ProxyTrackerBackend {
            trackers: trackers.to_owned(),
            conn_thread_handle: None,
            conn_thread_sender: None,
        };
        try!(backend.create_conn_thread());
        Ok(backend)
    }

    fn with_conn_thread_sender<F, T>(&self, callback: F) -> MogResult<T>
        where F: FnOnce(&Sender<ProxyRequest>) -> MogResult<T>
    {
        SENDER.with(|sender_cell| {
            let mut sender_opt = sender_cell.borrow_mut();

            if sender_opt.is_none() {
                *sender_opt = Some(try!(self.conn_thread_sender_clone()));
            }

            match *sender_opt {
                Some(ref mut sender) => callback(sender),
                None => Err(MogError::NoConnection),
            }
        })
    }

    fn conn_thread_sender_clone(&self) -> MogResult<Sender<ProxyRequest>> {
        let sender_mutex = try!(self.conn_thread_sender.as_ref().ok_or(MogError::NoConnection));
        match sender_mutex.lock() {
            Ok(locked) => Ok(locked.clone()),
            Err(poisoned) => {
                let locked = poisoned.into_inner();
                Ok(locked.clone())
            }
        }
    }

    pub fn create_conn_thread(&mut self) -> MogResult<()> {
        match self.conn_thread_handle {
            Some(..) => Ok(()),
            None => {
                let (tx, rx) = mpsc::channel::<ProxyRequest>();
                let trackers = self.trackers.clone();

                self.conn_thread_sender = Some(Mutex::new(tx));
                self.conn_thread_handle = Some(thread::spawn(move|| {
                    connection_thread(&trackers, rx);
                }));

                Ok(())
            }
        }
    }

    fn send_request<Req, Res>(&self, req: Req) -> MogResult<Res>
        where Req: Request + 'static, Res: Response + FromBytes
    {
        let (tx, rx) = mpsc::channel();

        try!(self.with_conn_thread_sender(|sender| {
            sender.send(ProxyRequest { inner: RequestInner::Real(Box::new(req)), respond: tx })
                .map_err(|e| MogError::from(e))
        }));

        rx.recv()
            .map_err(|e| MogError::from(e))
            .and_then(|pr| pr.inner)
            .and_then(|abstract_response| {
                // This is a horrible, horrible way to do this. I apologize.
                Res::from_bytes(abstract_response.to_urlencoded_string().as_bytes())
            })
    }
}

fn connection_thread(trackers: &[SocketAddr], requests: Receiver<ProxyRequest>) {
    let mut conn = MogClient::new(trackers);

    for proxy_request in requests.iter() {
        let response = match proxy_request.inner {
            RequestInner::Stop => {
                info!("Stopping and closing connection thread...");
                break;
            },
            RequestInner::Real(request) => {
                debug!("Sending request {:?} to {:?}", request, conn.peer_addr());
                let response = conn.request(request);
                debug!("Got response {:?} from {:?}", response, conn.peer_addr());
                response
            },
        };

        proxy_request.respond.send(ProxyResponse { inner: response })
            .unwrap_or_else(|err| {
                error!("Error sending response back to requesting thread: {}", err);
            });
    }
}

impl TrackerBackend for ProxyTrackerBackend {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<CreateDomain> {
        self.send_request(req.clone())
    }

    fn create_open(&self, req: &CreateOpen) -> MogResult<CreateOpenResponse> {
        self.send_request(req.clone())
    }

    fn create_close(&self, req: &CreateClose) -> MogResult<()> {
        self.send_request(req.clone())
    }

    fn get_paths(&self, req: &GetPaths) -> MogResult<GetPathsResponse> {
        self.send_request(req.clone())
    }
    
    fn file_info(&self, req: &FileInfo) -> MogResult<FileInfoResponse> {
        self.send_request(req.clone())
    }
    
    fn delete(&self, req: &Delete) -> MogResult<()> {
        self.send_request(req.clone())
    }

    fn rename(&self, req: &Rename) -> MogResult<()> {
        self.send_request(req.clone())
    }

    fn list_keys(&self, req: &ListKeys) -> MogResult<ListKeysResponse> {
        self.send_request(req.clone())
    }
}

pub trait AlternateFileFinder {
    fn file_info(&self, domain: &str, key: &str) -> MogResult<FileInfoResponse>;
    fn get_paths(&self, domain: &str, key: &str) -> MogResult<GetPathsResponse>;
}

pub struct ProxyWithAlternateBackend<F: AlternateFileFinder> {
    backend: ProxyTrackerBackend,
    finder: F,
}

impl<F: AlternateFileFinder> ProxyWithAlternateBackend<F> {
    pub fn new(backend: ProxyTrackerBackend, finder: F) -> ProxyWithAlternateBackend<F> {
        ProxyWithAlternateBackend {
            backend: backend,
            finder: finder,
        }
    }
}

impl<F: AlternateFileFinder + Send + Sync + 'static> TrackerBackend for ProxyWithAlternateBackend<F> {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<CreateDomain> {
        self.backend.send_request(req.clone())
    }

    fn create_open(&self, req: &CreateOpen) -> MogResult<CreateOpenResponse> {
        self.backend.send_request(req.clone())
    }

    fn create_close(&self, req: &CreateClose) -> MogResult<()> {
        self.backend.send_request(req.clone())
    }

    fn get_paths(&self, req: &GetPaths) -> MogResult<GetPathsResponse> {
        let response = self.backend.send_request(req.clone());
        debug!("In alternate: original response = {:?}", response);
        match response {
            o @ Err(MogError::UnknownKey(..)) | o @ Err(MogError::UnregDomain(..)) => {
                let alt_paths = self.finder.get_paths(&req.domain, &req.key);
                debug!("request: {:?} had error {:?}, alternate paths: {:?}", req, o, alt_paths);
                alt_paths.map_err(|_| o.unwrap_err())
            },
            r @ _ => r,
        }
    }
    
    fn file_info(&self, req: &FileInfo) -> MogResult<FileInfoResponse> {
        let response = self.backend.send_request(req.clone());
        debug!("In alternate: original response = {:?}", response);
        match response {
            o @ Err(MogError::UnknownKey(..)) | o @ Err(MogError::UnregDomain(..)) => {
                let alt_file_info = self.finder.file_info(&req.domain, &req.key);
                debug!("request: {:?} had error {:?}, alternate file: {:?}", req, o, alt_file_info);
                alt_file_info.map_err(|_| o.unwrap_err())
            },
            r @ _ => r,
        }
    }
    
    fn delete(&self, req: &Delete) -> MogResult<()> {
        self.backend.send_request(req.clone())
    }

    fn rename(&self, req: &Rename) -> MogResult<()> {
        self.backend.send_request(req.clone())
    }

    fn list_keys(&self, req: &ListKeys) -> MogResult<ListKeysResponse> {
        self.backend.send_request(req.clone())
    }
}

#[cfg(test)]
mod tests {
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
        let backend = ProxyTrackerBackend::new(&addr_list).unwrap();
        assert_eq!(addr_list, backend.trackers);
        assert!(matches!(backend.conn_thread_handle, Some(..)));
        assert!(matches!(backend.conn_thread_sender, Some(..)));
        assert!(matches!(backend.conn_thread_sender_clone(), Ok(..)));
        assert!(matches!(backend.with_conn_thread_sender(move|_| Ok(())), Ok(())))
    }

    #[test]
    fn can_stop_connection_thread() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let (req_tx, req_rx) = mpsc::channel();

        // Gosh, it would be great if I could time this out somehow by using Stable Rust...
        let conn_thread = thread::spawn(move|| connection_thread(&[ listener.local_addr().unwrap().clone() ], req_rx));
        stop_conn_thread(conn_thread, req_tx);
    }

    #[test]
    fn creates_conn_thread() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut backend = ProxyTrackerBackend::new(&[ listener.local_addr().unwrap() ]).unwrap();
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
