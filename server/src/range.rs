#![allow(unused_imports)]

use iron::headers::{Range, ByteRangeSpec, ContentRange, ContentRangeSpec, ContentLength};
use iron::modifier::{Modifier, Set};
use iron::status::Status;
use iron::{AroundMiddleware, Handler, IronError, IronResult, Request, Response};
use std::io::{Read, Cursor};

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

pub struct RangeMiddleware;

impl AroundMiddleware for RangeMiddleware {
    fn around(self, handler: Box<Handler>) -> Box<Handler> {
        Box::new(RangeHandler { handler: handler })
    }
}

#[derive(Debug)]
pub struct RangeModifier(Vec<ByteRangeSpec>);

impl RangeModifier {
    pub fn from_request(request: &Request) -> Option<RangeModifier> {
        request.headers.get::<Range>().and_then(|r| RangeModifier::from_range_header(r))
    }

    pub fn from_range_header(header: &Range) -> Option<RangeModifier> {
        match header {
            &Range::Bytes(ref spec_vec) => Some(RangeModifier(spec_vec.clone())),
            _ => None,
        }
    }
}

impl Modifier<Response> for RangeModifier {
    fn modify(self, _response: &mut Response) {
        error!("!!!!!!!!!!! Not modifying response for range: {:?}", self);
    }
}

// fn modify_response_for_ranges(response: &mut Response, ranges: &[ByteRangeSpec]) -> IronResult<()> {
//     match ranges.len() {
//         0 => Ok(()), // Just return the result unmodified... ?
//         1 => modify_response_for_range(response, &ranges[0]),
//         _ => Ok(()), // Don't bother handling multiple-range requests for now.
//     }
// }

// fn modify_response_for_range(response: &mut Response, range: &ByteRangeSpec) -> IronResult<()> {
//     if response.body.is_none() {
//         return Ok(())
//     }

//     let mut old_body = response.body.take().unwrap();
//     let mut body_vec = vec![];
//     match old_body.read_to_end(&mut body_vec) {
//         Err(e) => {
//             return Err(IronError::new(e, (Status::InternalServerError,)));
//         },
//         _ => {}
//     }

//     debug!("Limiting response to range: {:?} of {:?}", range, body_vec.len());

//     response.status = Some(Status::PartialContent);
//     match range {
//         &ByteRangeSpec::FromTo(from, to) => {
//             let req_len = to - from + 1;
//             let new_body_vec: Vec<u8> = body_vec.iter()
//                 .skip(from as usize)
//                 .take(req_len as usize)
//                 .cloned()
//                 .collect();

//             let res_from = from;
//             let res_to = res_from + (new_body_vec.len() as u64) - 1;
//             response.headers.set(ContentRange(ContentRangeSpec::Bytes {
//                 range: Some((res_from, res_to)),
//                 instance_length: Some(body_vec.len() as u64),
//             }));
//             response.headers.set(ContentLength(new_body_vec.len() as u64));
//             response.body = Some(Box::new(Cursor::new(new_body_vec)));
//             Ok(())
//         },
//         &ByteRangeSpec::AllFrom(from) => {
//             let new_body_vec: Vec<u8> = body_vec.iter()
//                 .skip(from as usize)
//                 .cloned()
//                 .collect();

//             let res_from = from;
//             let res_to = res_from + (new_body_vec.len() as u64) - 1;

//             response.headers.set(ContentRange(ContentRangeSpec::Bytes {
//                 range: Some((res_from, res_to)),
//                 instance_length: Some(body_vec.len() as u64),
//             }));
//             response.headers.set(ContentLength(new_body_vec.len() as u64));
//             response.body = Some(Box::new(Cursor::new(new_body_vec)));
//             Ok(())
//         },
//         &ByteRangeSpec::Last(n) => {
//             if n > (body_vec.len() as u64) {
//                 response.headers.set(ContentRange(ContentRangeSpec::Bytes {
//                     range: Some((0, (body_vec.len() as u64) - 1)),
//                     instance_length: Some(body_vec.len() as u64),
//                 }));
//                 response.headers.set(ContentLength(body_vec.len() as u64));
//                 response.body = Some(Box::new(Cursor::new(body_vec)));
//                 Ok(())
//             } else {
//                 let req_from = (body_vec.len() as u64) - n;
//                 let new_body_vec: Vec<u8> = body_vec.iter()
//                     .skip(req_from as usize)
//                     .cloned()
//                     .collect();

//                 let res_from = req_from;
//                 let res_to = res_from + (new_body_vec.len() as u64) - 1;

//                 response.headers.set(ContentRange(ContentRangeSpec::Bytes {
//                     range: Some((res_from, res_to)),
//                     instance_length: Some(body_vec.len() as u64),
//                 }));
//                 response.headers.set(ContentLength(new_body_vec.len() as u64));
//                 response.body = Some(Box::new(Cursor::new(new_body_vec)));
//                 Ok(())
//             }
//         },
//     }
// }
