use mogilefs_common::requests::*;
use mogilefs_common::{MogError, MogResult, Request, Response, FromBytes};
use std::str;
use super::super::backend::TrackerBackend;

pub mod evented;
pub mod threaded;

/// Something that can be handled by the tracker, i.e, a handler for a
/// Request. Responsible for calling the appropriate method on the
/// Backend and turning the response in to a Response.
trait Handlable<B: TrackerBackend>: Request {
    fn handle(&self, backend: &B) -> MogResult<Response>;
}

/// The tracker object.
pub struct Tracker<B: TrackerBackend> {
    backend: B,
}

impl<B: TrackerBackend> Tracker<B> {
    pub fn new(backend: B) -> Tracker<B> {
        Tracker {
            backend: backend,
        }
    }

    /// Handle a request.
    pub fn handle_bytes(&self, request_bytes: &[u8]) -> MogResult<Response> {
        let request_result = request_from_bytes(request_bytes);
        info!("request = {:?}", request_result);
        let response_result = request_result.and_then(|req| req.handle(&self.backend));
        info!("response = {:?}", response_result);
        response_result
    }
}

/// A factory function to call the convert the bytes to the
/// appropriate Request type.
fn request_from_bytes<B: TrackerBackend>(bytes: &[u8]) -> MogResult<Box<Handlable<B>>> {
    let mut toks = bytes.split(|&b| b == b' ');
    let op = toks.next();
    let args = toks.next().unwrap_or(&[]);

    match op.map(|bs| str::from_utf8(bs)) {
        Some(Ok("create_domain")) => coerce_request(CreateDomain::from_bytes(args)),

        Some(Ok("create_open"))   => coerce_request(CreateOpen::from_bytes(args)),
        Some(Ok("create_close"))  => coerce_request(CreateClose::from_bytes(args)),
        Some(Ok("get_paths"))     => coerce_request(GetPaths::from_bytes(args)),
        Some(Ok("file_info"))     => coerce_request(FileInfo::from_bytes(args)),
        Some(Ok("rename"))        => coerce_request(Rename::from_bytes(args)),
        Some(Ok("updateclass"))   => coerce_request(UpdateClass::from_bytes(args)),
        Some(Ok("delete"))        => coerce_request(Delete::from_bytes(args)),
        Some(Ok("list_keys"))     => coerce_request(ListKeys::from_bytes(args)),

        Some(Ok("noop"))          => coerce_request(Noop::from_bytes(args)),

        Some(Ok(""))     => Err(MogError::UnknownCommand(None)),
        Some(Ok(string)) => Err(MogError::UnknownCommand(Some(string.to_string()))),
        Some(Err(utf8e)) => Err(MogError::Utf8(utf8e)),
        None => Err(MogError::UnknownCommand(None)),
    }
}

impl<B: TrackerBackend> Handlable<B> for CreateDomain {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        try!(backend.create_domain(&self.domain));
        Ok(Response::new(vec![ ("domain".to_string(), self.domain.clone()) ]))
    }
}

impl<B: TrackerBackend> Handlable<B> for CreateOpen {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        let urls = try!(backend.create_open(&self.domain, &self.key));
        let mut response_args = vec![];
        response_args.push(("dev_count".to_string(), urls.len().to_string()));
        for (i, url) in urls.iter().enumerate() {
            response_args.push((format!("devid_{}", i+1), (i+1).to_string()));
            response_args.push((format!("path_{}", i+1), url.to_string()));
        }
        Ok(Response::new(response_args))
    }
}

impl<B: TrackerBackend> Handlable<B> for CreateClose {
    fn handle(&self, _backend: &B) -> MogResult<Response> {
        // There actually are implementations of this on the backend,
        // but they don't do anything at the moment, and there's not
        // much point in writing code here if it's not going to be
        // used. We'll just leave this blank for now.
        Ok(Response::new(vec![]))
    }
}

// request = "get_paths domain=rn_development_private&key=Song/512428/image&noverify=1&zone=\r\n"
// response = "OK paths=1&path1=http://127.0.0.1:7500/dev1/0/000/000/0000000109.fid\r\n"
impl<B: TrackerBackend> Handlable<B> for GetPaths {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        let paths = try!(backend.get_paths(&self.domain, &self.key));
        let mut response_args = vec![ ("paths".to_string(), paths.len().to_string()) ];
        for (i, url) in paths.iter().enumerate() {
            response_args.push((format!("path{}", i+1), url.to_string()));
        }
        Ok(Response::new(response_args))
    }
}

// request = "file_info domain=rn_development_private&key=Song/23198312/image\r\n"
// response = "OK length=4142596&class=song_replicated&devcount=1&key=Song/23198312/image&fid=264&domain=rn_development_private\r\n"
impl<B: TrackerBackend> Handlable<B> for FileInfo {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        let meta = try!(backend.file_info(&self.domain, &self.key));

        let response_args = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("length".to_string(), meta.size.to_string()),
        };

        Ok(Response::new(response_args))
    }
}

// request = "rename domain=rn_development_private&from_key=Song/512428/image&to_key=Song/512428/image/1\r\n"
// response = "OK \r\n"
// request = "rename domain=rn_development_private&from_key=Song/9381/image&to_key=Song/512428/image/1\r\n"
// response = "ERR key_exists Target+key+name+already+exists%3B+can%27t+overwrite.\r\n"
// request = "rename domain=rn_development_private&from_key=Song/512428/image&to_key=Song/512428/image/1\r\n"
// response = "ERR unknown_key unknown_key\r\n"
impl<B: TrackerBackend> Handlable<B> for Rename {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        try!(backend.rename(&self.domain, &self.from_key, &self.to_key));
        Ok(Response::new(vec![]))
    }
}

impl<B: TrackerBackend> Handlable<B> for UpdateClass {
    fn handle(&self, _backend: &B) -> MogResult<Response> {
        // We don't support classes at the moment; just smile and nod.
        Ok(Response::new(vec![]))
    }
}

impl<B: TrackerBackend> Handlable<B> for Delete {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        try!(backend.delete(&self.domain, &self.key));
        Ok(Response::new(vec![]))
    }
}

impl<B: TrackerBackend> Handlable<B> for ListKeys {
    fn handle(&self, backend: &B) -> MogResult<Response> {
        let keys = try!(backend.list_keys(
            &self.domain,
            self.prefix.as_ref().map(|p| p as &str),
            self.after.as_ref().map(|a| a as &str),
            self.limit.map(|lim| lim as usize)));

        let mut response_args = vec![ ("key_count".to_string(), keys.len().to_string()) ];
        for (i, key) in keys.iter().enumerate() {
            response_args.push((format!("key_{}", i+1), key.to_string()));
            if i == keys.len() - 1 {
                response_args.push(("next_after".to_string(), key.to_string()));
            }
        }

        Ok(Response::new(response_args))
    }
}

impl<B: TrackerBackend> Handlable<B> for Noop {
    fn handle(&self, _backend: &B) -> MogResult<Response> {
        Ok(Response::new(vec![]))
    }
}

/// Convert the Result of a parsed Request in to a Result of a boxed
/// version of that Request, casting to ignore the concrete type of
/// the Request.
fn coerce_request<B, T>(req: MogResult<T>) -> MogResult<Box<Handlable<B>>>
    where B: TrackerBackend, T: Handlable<B> + FromBytes + 'static
{
    req.map(|r| Box::new(r) as Box<Handlable<B>>)
}
