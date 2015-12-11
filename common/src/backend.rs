use hyper::Client as HyperClient;
use hyper::status::StatusCode;
use rand;
use std::io::Read;
use super::error::{MogError, MogResult};
use super::request::{Request, Response};
use super::requests::*;

/// A backend for the trackers.
pub trait Backend: Send + Sync {
    fn create_domain(&self, &CreateDomain) -> MogResult<CreateDomain>;
    fn create_open  (&self, &CreateOpen)   -> MogResult<CreateOpenResponse>;
    fn create_close (&self, &CreateClose)  -> MogResult<()>;
    fn create_class (&self, &CreateClass)  -> MogResult<CreateClassResponse>;
    fn get_paths    (&self, &GetPaths)     -> MogResult<GetPathsResponse>;
    fn file_info    (&self, &FileInfo)     -> MogResult<FileInfoResponse>;
    fn delete       (&self, &Delete)       -> MogResult<()>;
    fn rename       (&self, &Rename)       -> MogResult<()>;
    fn list_keys    (&self, &ListKeys)     -> MogResult<ListKeysResponse>;

    fn handle<R: Request + ?Sized>(&self, request: &R) -> MogResult<Response> where Self: Sized {
        request.perform(self)
    }

    fn store_file<R: Read>(&self, domain: String, key: String, class: Option<String>, data: &mut R) -> MogResult<()> where Self: Sized {
        // Register the file with MogileFS, and ask it where we can store it.
        let open_req = CreateOpen { domain: domain.clone(), class: class, key: key.clone(), multi_dest: true, size: None };
        let open_res = try!(self.create_open(&open_req));

        // Choose at random one of the places MogileFS suggests.
        let mut rng = rand::thread_rng();
        let &&(ref devid, ref path) = try!(rand::sample(&mut rng, open_res.paths.iter(), 1).first().ok_or(MogError::NoPath));

        debug!("Storing data for {:?} to {}", key, path);

        // Upload the file.
        let client = HyperClient::new();
        let put_res = try!{
            client.put(path.clone())
                .body(data)
                .send()
                .map_err(|e| MogError::StorageError(Some(format!("Could not store to {}: {}", path, e))))
        };

        match &put_res.status {
            &StatusCode::Ok | &StatusCode::Created => {},
            _ => return Err(MogError::StorageError(Some(format!("Bad response from storage server: {:?}", put_res)))),
        }

        // Tell MogileFS where we uploaded the file to, and return the
        // result of telling it so.
        self.create_close(&CreateClose {
            domain: domain,
            key: key,
            fid: open_res.fid,
            devid: *devid,
            path: path.clone(),
            checksum: None,
        })
    }
}

/// Middleware that wraps the handling of a Request.
pub trait AroundMiddleware {
    fn around(self, backend: Box<Backend>) -> Box<Backend>;
}

/// A middleware stack wrapping a Backend.
pub struct BackendStack {
    backend: Option<Box<Backend>>,
}

impl BackendStack {
    pub fn new<B: Backend + 'static>(backend: B) -> BackendStack {
        BackendStack {
            backend: Some(Box::new(backend) as Box<Backend>),
        }
    }

    pub fn around<A: AroundMiddleware>(&mut self, around: A) {
        let mut backend = self.backend.take().unwrap();
        backend = around.around(backend);
        self.backend = Some(backend);
    }
}

impl Backend for BackendStack {
    fn create_domain(&self, req: &CreateDomain) -> MogResult<CreateDomain> {
        self.backend.as_ref().unwrap().create_domain(req)
    }

    fn create_open(&self, req: &CreateOpen) -> MogResult<CreateOpenResponse> {
        self.backend.as_ref().unwrap().create_open(req)
    }

    fn create_close(&self, req: &CreateClose) -> MogResult<()> {
        self.backend.as_ref().unwrap().create_close(req)
    }

    fn create_class(&self, req: &CreateClass) -> MogResult<CreateClassResponse> {
        self.backend.as_ref().unwrap().create_class(req)
    }

    fn get_paths(&self, req: &GetPaths) -> MogResult<GetPathsResponse> {
        self.backend.as_ref().unwrap().get_paths(req)
    }

    fn file_info(&self, req: &FileInfo) -> MogResult<FileInfoResponse> {
        self.backend.as_ref().unwrap().file_info(req)
    }

    fn delete(&self, req: &Delete) -> MogResult<()> {
        self.backend.as_ref().unwrap().delete(req)
    }

    fn rename(&self, req: &Rename) -> MogResult<()> {
        self.backend.as_ref().unwrap().rename(req)
    }

    fn list_keys(&self, req: &ListKeys) -> MogResult<ListKeysResponse> {
        self.backend.as_ref().unwrap().list_keys(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::error::MogResult;
    use super::super::requests::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use url::Url;

    struct CountingBackend {
        create_domain: AtomicUsize,
        create_open: AtomicUsize,
        create_close: AtomicUsize,
        create_class: AtomicUsize,
        get_paths: AtomicUsize,
        file_info: AtomicUsize,
        delete: AtomicUsize,
        rename: AtomicUsize,
        list_keys: AtomicUsize,
    }

    impl CountingBackend {
        fn new() -> CountingBackend {
            CountingBackend {
                create_domain: AtomicUsize::new(0),
                create_open: AtomicUsize::new(0),
                create_close: AtomicUsize::new(0),
                create_class: AtomicUsize::new(0),
                get_paths: AtomicUsize::new(0),
                file_info: AtomicUsize::new(0),
                delete: AtomicUsize::new(0),
                rename: AtomicUsize::new(0),
                list_keys: AtomicUsize::new(0),
            }
        }

        fn increment(&self, counter: &AtomicUsize) {
            let mut tries: u8 = 0;
            loop {
                let current = counter.load(Ordering::Relaxed);
                let previous = counter.compare_and_swap(current, current + 1, Ordering::Relaxed);
                tries += 1;
                if current == previous || tries > 10 {
                    break;
                }
            }
        }
    }

    impl Backend for CountingBackend {
        fn create_domain(&self, _: &CreateDomain) -> MogResult<CreateDomain> {
            self.increment(&self.create_domain);
            Ok(CreateDomain {
                domain: "test_domain".to_string(),
            })
        }

        fn create_open(&self, _: &CreateOpen) -> MogResult<CreateOpenResponse> {
            self.increment(&self.create_open);
            Ok(CreateOpenResponse {
                fid: 1000,
                paths: vec![(1, Url::parse("http://127.0.0.1:7099/fid/1000/path.fid").unwrap())],
            })
        }

        fn create_close(&self, _: &CreateClose) -> MogResult<()> {
            self.increment(&self.create_close);
            Ok(())
        }

        fn create_class(&self, _: &CreateClass) -> MogResult<CreateClassResponse> {
            self.increment(&self.create_class);
            Ok(CreateClassResponse {
                domain: "test_domain".to_string(),
                class: "test_class".to_string(),
                mindevcount: 2,
            })
        }

        fn get_paths(&self, _: &GetPaths) -> MogResult<GetPathsResponse> {
            self.increment(&self.get_paths);
            Ok(GetPathsResponse(vec![ Url::parse("http://127.0.0.1:7099/test/key/1000/file.fid").unwrap() ]))
        }

        fn file_info(&self, _: &FileInfo) -> MogResult<FileInfoResponse> {
            self.increment(&self.file_info);
            Ok(FileInfoResponse {
                fid: 1000,
                devcount: 2,
                length: 1048576,
                domain: "test_domain".to_string(),
                class: "test_class".to_string(),
                key: "test/key/1000".to_string(),
            })
        }

        fn delete(&self, _: &Delete) -> MogResult<()> {
            self.increment(&self.delete);
            Ok(())
        }

        fn rename(&self, _: &Rename) -> MogResult<()> {
            self.increment(&self.rename);
            Ok(())
        }

        fn list_keys(&self, _: &ListKeys) -> MogResult<ListKeysResponse> {
            self.increment(&self.list_keys);
            Ok(ListKeysResponse(vec![ "test/key/1000".to_string() ]))
        }
    }

    #[test]
    fn test_handle() {
        let backend = CountingBackend::new();

        assert_eq!(0, backend.create_domain.load(Ordering::Relaxed));
        let response = backend.handle(&CreateDomain { domain: "test_domain".to_string() });
        assert!(response.is_ok());
        assert_eq!(1, backend.create_domain.load(Ordering::Relaxed));
    }

    #[test]
    fn test_store_file() {
        let backend = CountingBackend::new();
        let mut content = Cursor::new("File content");

        assert_eq!(0, backend.create_open.load(Ordering::Relaxed));
        assert_eq!(0, backend.create_close.load(Ordering::Relaxed));

        let response = backend.store_file(
            "test_domain".to_string(),
            "test/key/1000".to_string(),
            None, &mut content);
        println!("response = {:?}", response);
        assert!(response.is_ok());

        assert_eq!(1, backend.create_open.load(Ordering::Relaxed));
        assert_eq!(1, backend.create_close.load(Ordering::Relaxed));
    }
}
