use mogilefs_common::requests::*;
use mogilefs_common::{MogError, MogResult, Request, Response, FromBytes};
use std::str;
use super::super::backend::TrackerBackend;

pub mod evented;
pub mod threaded;

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
            Some(Ok("create_domain")) => self.handle_request(CreateDomain::from_bytes(args), &B::create_domain),
            Some(Ok("create_open"))   => self.handle_request(CreateOpen::from_bytes(args),   &B::create_open),
            Some(Ok("create_close"))  => self.handle_request(CreateClose::from_bytes(args),  &B::create_close),
            Some(Ok("file_info"))     => self.handle_request(FileInfo::from_bytes(args),     &B::file_info),
            Some(Ok("get_paths"))     => self.handle_request(GetPaths::from_bytes(args),     &B::get_paths),
            Some(Ok("rename"))        => self.handle_request(Rename::from_bytes(args),       &B::rename),
            Some(Ok("updateclass"))   => self.handle_request(UpdateClass::from_bytes(args),  &empty_handler::<B, UpdateClass>),
            Some(Ok("delete"))        => self.handle_request(Delete::from_bytes(args),       &B::delete),
            Some(Ok("list_keys"))     => self.handle_request(ListKeys::from_bytes(args),     &B::list_keys),
            Some(Ok("noop"))          => self.handle_request(Noop::from_bytes(args),         &empty_handler::<B, Noop>),

            Some(Ok(""))     => Err(MogError::UnknownCommand(None)),
            Some(Ok(string)) => Err(MogError::UnknownCommand(Some(string.to_string()))),
            Some(Err(utf8e)) => Err(MogError::Utf8(utf8e)),
            None => Err(MogError::UnknownCommand(None)),
        }
    }

    pub fn handle_request<Req, Res, F>(&self, request: MogResult<Req>, handler_fn: &F) -> MogResult<Box<Response>>
        where Req: Request + Sized + 'static, Res: Response + Sized + 'static, F: Fn(&B, &Req) -> MogResult<Res>
    {
        info!("request = {:?}", request);
        let response = request.and_then(|req| handler_fn(&self.backend, &req).map(|res| Box::new(res) as Box<Response>));
        info!("response = {:?}", response);
        response
    }
}

fn empty_handler<B: TrackerBackend, Req: Request + Sized + 'static>(_backend: &B, _request: &Req) -> MogResult<()> {
    Ok(())
}
