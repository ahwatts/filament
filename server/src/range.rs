//! Iron middleware and modifier for handling HTTP Range headers.

use iron::headers::{Range, ByteRangeSpec, ContentRange, ContentRangeSpec, ContentLength};
use iron::modifier::{Modifier, Set};
use iron::response::ResponseBody;
use iron::status::Status;
use iron::{typemap, AroundMiddleware, Handler, IronResult, Request, Response};
use std::io::{self, Read, Cursor};
use std::iter;
use plugin::{Plugin, Pluggable};

struct RangeHandler<H: Handler> {
    handler: H
}

impl<H: Handler> Handler for RangeHandler<H> {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let modifier = RangeModifier::from_request(request);
        let response_rslt = self.handler.handle(request);
        match (response_rslt, modifier) {
            (Ok(response), Some(range_mod)) => Ok(response.set(range_mod)),
            (response_rslt, _) => response_rslt,
        }
    }
}

/// "Around" middleware which handles a `Range` header.
///
/// Captures the `Range` header from the request, and then breaks up
/// the response into the specified ranges, assuming the response is
/// the whole content.
pub struct RangeMiddleware;

impl AroundMiddleware for RangeMiddleware {
    fn around(self, handler: Box<Handler>) -> Box<Handler> {
        Box::new(RangeHandler { handler: handler })
    }
}

/// A `Modifier` for `Response`s which slices up the body of the
/// response into the range(s) specified in the request.
///
/// Currently does not handle multiple `Range`s specified in the
/// request. If there isn't exactly one range in the request, this
/// modifier returns the original response, unmodified.
#[derive(Debug)]
pub struct RangeModifier(Vec<ByteRangeSpec>);

impl RangeModifier {
    /// Create a `RangeModifier` from the request.
    ///
    /// If there is no `Range` header present, or the `Range` header
    /// does not use bytes as its unit, returns `None`.
    pub fn from_request(request: &Request) -> Option<RangeModifier> {
        request.headers.get::<Range>().and_then(|r| RangeModifier::from_range_header(r))
    }

    /// Create a `RangeModifier` from a `Range` header.
    ///
    /// If the `Range` header does not use bytes as its unit, returns
    /// `None`.
    pub fn from_range_header(header: &Range) -> Option<RangeModifier> {
        match header {
            &Range::Bytes(ref spec_vec) => Some(RangeModifier(spec_vec.clone())),
            _ => None,
        }
    }
}

impl Modifier<Response> for RangeModifier {
    fn modify(self, response: &mut Response) {
        if self.0.len() != 1 { return; }
        let range = self.0.into_iter().next().unwrap();

        let orig_body = response.get::<BodyExtractor>();
        if orig_body.is_err() { return; }
        let orig_body = orig_body.unwrap();

        debug!("Limiting response to range: {:?} of {:?}", range, orig_body.len());

        response.status = Some(Status::PartialContent);
        match range {
            ByteRangeSpec::FromTo(from, to) => modify_from_to(response, orig_body, from, to),
            ByteRangeSpec::AllFrom(from) => modify_all_from(response, orig_body, from),
            ByteRangeSpec::Last(to) => modify_last(response, orig_body, to),
        }
    }
}

fn modify_from_to(response: &mut Response, orig_body: Vec<u8>, from: u64, to: u64) {
    let orig_len = orig_body.len();
    let req_len = to - from + 1;
    let new_body: Vec<u8> = orig_body.into_iter()
        .skip(from as usize)
        .take(req_len as usize)
        .collect();

    let res_from = from;
    let res_to = res_from + (new_body.len() as u64) - 1;
    response.headers.set(ContentRange(ContentRangeSpec::Bytes {
        range: Some((res_from, res_to)),
        instance_length: Some(orig_len as u64),
    }));

    response.set_mut(new_body);
}

fn modify_all_from(response: &mut Response, orig_body: Vec<u8>, from: u64) {
    let orig_len = orig_body.len();
    let new_body: Vec<u8> = orig_body.into_iter()
        .skip(from as usize)
        .collect();

    let res_from = from;
    let res_to = res_from + (new_body.len() as u64) - 1;
    response.headers.set(ContentRange(ContentRangeSpec::Bytes {
        range: Some((res_from, res_to)),
        instance_length: Some(orig_len as u64),
    }));

    response.set_mut(new_body);
}

fn modify_last(response: &mut Response, orig_body: Vec<u8>, to: u64) {
    let orig_len = orig_body.len();

    if to > (orig_len as u64) {
        response.headers.set(ContentRange(ContentRangeSpec::Bytes {
            range: Some((0, (orig_len as u64) - 1)),
            instance_length: Some(orig_len as u64),
        }));

        response.set_mut(orig_body);
    } else {
        let req_from = (orig_len as u64) - to;
        let new_body: Vec<u8> = orig_body.into_iter()
            .skip(req_from as usize)
            .collect();

        let res_from = req_from;
        let res_to = res_from + (new_body.len() as u64) - 1;
        response.headers.set(ContentRange(ContentRangeSpec::Bytes {
            range: Some((res_from, res_to)),
            instance_length: Some(orig_len as u64),
        }));

        response.set_mut(new_body);
    }
}

struct BodyExtractor;

impl typemap::Key for BodyExtractor {
    type Value = Vec<u8>;
}

impl Plugin<Response> for BodyExtractor {
    type Error = io::Error;

    fn eval(response: &mut Response) -> Result<Vec<u8>, io::Error> {
        match &mut response.body {
            &mut Some(ref mut writer) => {
                let content_length = response.headers.get::<ContentLength>().map(|h| h.0).unwrap_or(0);
                let mut body: Vec<u8> = iter::repeat(0u8).take(content_length as usize).collect();
                try!(writer.write_body(&mut ResponseBody::new(Cursor::new(body.as_mut()))));
                Ok(body)
            },
            &mut None => {
                Ok(vec![])
            }
        }
    }
}
