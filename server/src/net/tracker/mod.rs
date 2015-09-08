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
    fn handle(&self, backend: &B) -> MogResult<Box<Response>>;
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
    pub fn handle_bytes(&self, request_bytes: &[u8]) -> MogResult<Box<Response>> {
        let mut toks = request_bytes.split(|&b| b == b' ');
        let op = toks.next();
        let args = toks.next().unwrap_or(&[]);

        match op.map(|bs| str::from_utf8(bs)) {
            Some(Ok("create_domain")) => CreateDomain::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("create_open")) => CreateOpen::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("create_close")) => CreateClose::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("file_info")) => FileInfo::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("get_paths")) => GetPaths::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("rename")) => Rename::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("updateclass")) => UpdateClass::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("delete")) => Delete::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("list_keys")) => ListKeys::from_bytes(args).and_then(|r| r.handle(&self.backend)),
            Some(Ok("noop")) => Noop::from_bytes(args).and_then(|r| r.handle(&self.backend)),

            Some(Ok(""))     => Err(MogError::UnknownCommand(None)),
            Some(Ok(string)) => Err(MogError::UnknownCommand(Some(string.to_string()))),
            Some(Err(utf8e)) => Err(MogError::Utf8(utf8e)),
            None => Err(MogError::UnknownCommand(None)),
        }
    }
}

impl<B: TrackerBackend> Handlable<B> for CreateDomain {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.create_domain(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for CreateOpen {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.create_open(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for CreateClose {
    fn handle(&self, _backend: &B) -> MogResult<Box<Response>> {
        // There actually are implementations of this on the backend,
        // but they don't do anything at the moment, and there's not
        // much point in writing code here if it's not going to be
        // used. We'll just leave this blank for now.
        Ok(Box::new(()) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for GetPaths {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.get_paths(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for FileInfo {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.file_info(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for Rename {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.rename(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for UpdateClass {
    fn handle(&self, _backend: &B) -> MogResult<Box<Response>> {
        Ok(Box::new(()) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for Delete {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.delete(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for ListKeys {
    fn handle(&self, backend: &B) -> MogResult<Box<Response>> {
        backend.list_keys(self).map(|r| Box::new(r) as Box<Response>)
    }
}

impl<B: TrackerBackend> Handlable<B> for Noop {
    fn handle(&self, _backend: &B) -> MogResult<Box<Response>> {
        Ok(Box::new(()) as Box<Response>)
    }
}
