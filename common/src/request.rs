use std::fmt::Debug;
use super::args_hash::ArgsHash;
use super::error::{MogError, MogResult};
use super::util::{FromBytes, ToArgs, ToUrlencodedString};

pub trait Request: Debug + ToArgs {
    type ResponseType: Response;
    fn op(&self) -> &'static str;
}

pub trait Response: Debug + ToArgs + Sync + Send {}

impl<R: Response> Response for Box<R> {}

impl Response for Box<Response> {}

impl Response for () {}

pub trait Renderable {
    fn render(&self) -> String;
}

impl<R: Response> Renderable for R {
    fn render(&self) -> String {
        format!("OK {}", self.to_urlencoded_string())
    }
}

#[derive(Debug)]
pub struct CreateDomain {
    pub domain: String,
}

impl Request for CreateDomain {
    type ResponseType = ();
    fn op(&self) -> &'static str { "create_domain" }
}

impl FromBytes for CreateDomain {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateDomain> {
        let mut args = ArgsHash::from_bytes(bytes);
        let domain = try!(args.extract_domain());

        Ok(CreateDomain {
            domain: domain,
        })
    }
}

impl ToArgs for CreateDomain {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
        }
    }
}

// pub mod types {
//     use super::Request;
//     use super::super::args_hash::ArgsHash;
//     use super::super::util::{ToArgs, FromBytes};
//     use super::super::error::{MogError, MogResult};
//     use url::Url;

//     macro_rules! request_type {
//         ( $name:ident $op:expr ) => {
//             #[derive(Debug)]
//             pub struct $name;
//             impl Request for $name {
//                 fn op(&self) -> &'static str {
//                     $op
//                 }
//             }
//         };
//         ( $name:ident $op:expr { $( $f:ident : $t:ty ),* } ) => {
//             #[derive(Debug)]
//             pub struct $name {
//                 $( pub $f: $t, )*
//             }
//             impl Request for $name {
//                 fn op(&self) -> &'static str {
//                     $op
//                 }
//             }
//         };
//     }

//     request_type!{ CreateDomain "create_domain" { domain: String } }
//     request_type!{ CreateOpen   "create_open"   { domain: String, key: String, multi_dest: bool, size: Option<u64> } }
//     request_type!{ CreateClose  "create_close"  { domain: String, key: String, fid: u64, devid: u64, path: Url, checksum: Option<String> } }
//     request_type!{ GetPaths     "get_paths"     { domain: String, key: String, noverify: bool, pathcount: Option<u64> } }
//     request_type!{ FileInfo     "file_info"     { domain: String, key: String } }
//     request_type!{ Rename       "rename"        { domain: String, from_key: String, to_key: String } }
//     request_type!{ UpdateClass  "updateclass"   { domain: String, key: String, new_class: String } }
//     request_type!{ Delete       "delete"        { domain: String, key: String } }
//     request_type!{ ListKeys     "list_keys"     { domain: String, prefix: Option<String>, after: Option<String>, limit: Option<u64> } }
//     request_type!{ Noop         "noop" }

//     impl FromBytes for CreateDomain {
//         fn from_bytes(bytes: &[u8]) -> MogResult<CreateDomain> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());

//             Ok(CreateDomain {
//                 domain: domain,
//             })
//         }
//     }

//     impl ToArgs for CreateDomain {
//         fn to_args(&self) -> Vec<(String, String)> {
//             vec!{
//                 ("domain".to_string(), self.domain.clone()),
//             }
//         }
//     }

//     impl FromBytes for CreateOpen {
//         fn from_bytes(bytes: &[u8]) -> MogResult<CreateOpen> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let key = try!(args.extract_key());
//             let multi_dest = args.extract_bool_value("multi_dest", false);
//             let size = args.extract_optional_int("size");

//             Ok(CreateOpen {
//                 domain: domain,
//                 key: key,
//                 multi_dest: multi_dest,
//                 size: size,
//             })
//         }
//     }

//     impl ToArgs for CreateOpen {
//         fn to_args(&self) -> Vec<(String, String)> {
//             let mut rv = vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("key".to_string(), self.key.clone()),
//                 ("multi_dest".to_string(), self.multi_dest.to_string()),
//             };

//             if self.size.is_some() {
//                 rv.push(("size".to_string(), self.size.clone().unwrap().to_string()));
//             }

//             rv
//         }
//     }

//     impl FromBytes for CreateClose {
//         fn from_bytes(bytes: &[u8]) -> MogResult<CreateClose> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let key = try!(args.extract_key());
//             let fid = try!(args.extract_required_int("fid", MogError::NoFid));
//             let devid = try!(args.extract_required_int("devid", MogError::NoDevid));
//             let path = try!(args.extract_required_url("path", MogError::NoPath));
//             let checksum = args.extract_optional_string("checksum");

//             Ok(CreateClose {
//                 domain: domain,
//                 key: key,
//                 fid: fid,
//                 devid: devid,
//                 path: path,
//                 checksum: checksum,
//             })
//         }
//     }

//     impl ToArgs for CreateClose {
//         fn to_args(&self) -> Vec<(String, String)> {
//             let mut rv = vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("key".to_string(), self.key.clone()),
//                 ("fid".to_string(), self.fid.to_string()),
//                 ("devid".to_string(), self.devid.to_string()),
//                 ("path".to_string(), self.path.to_string()),
//             };

//             if self.checksum.is_some() {
//                 rv.push(("checksum".to_string(), self.checksum.clone().unwrap()));
//             }

//             rv
//         }
//     }

//     impl FromBytes for GetPaths {
//         fn from_bytes(bytes: &[u8]) -> MogResult<GetPaths> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let key = try!(args.extract_key());
//             let noverify = args.extract_bool_value("noverify", false);
//             let pathcount = args.extract_optional_int("pathcount");

//             Ok(GetPaths {
//                 domain: domain,
//                 key: key,
//                 noverify: noverify,
//                 pathcount: pathcount,
//             })
//         }
//     }

//     impl ToArgs for GetPaths {
//         fn to_args(&self) -> Vec<(String, String)> {
//             let mut rv = vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("key".to_string(), self.key.clone()),
//                 ("noverify".to_string(), self.noverify.to_string()),
//             };

//             if self.pathcount.is_some() {
//                 rv.push(("pathcount".to_string(), self.pathcount.clone().unwrap().to_string()));
//             }

//             rv
//         }
//     }

//     impl FromBytes for FileInfo {
//         fn from_bytes(bytes: &[u8]) -> MogResult<FileInfo> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let key = try!(args.extract_key());

//             Ok(FileInfo {
//                 domain: domain,
//                 key: key,
//             })
//         }
//     }

//     impl ToArgs for FileInfo {
//         fn to_args(&self) -> Vec<(String, String)> {
//             vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("key".to_string(), self.key.clone()),
//             }
//         }
//     }

//     impl FromBytes for Rename {
//         fn from_bytes(bytes: &[u8]) -> MogResult<Rename> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let from_key = try!(args.extract_required_string("from_key", MogError::NoKey));
//             let to_key = try!(args.extract_required_string("to_key", MogError::NoKey));

//             Ok(Rename {
//                 domain: domain,
//                 from_key: from_key,
//                 to_key: to_key,
//             })
//         }
//     }

//     impl ToArgs for Rename {
//         fn to_args(&self) -> Vec<(String, String)> {
//             vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("from_key".to_string(), self.from_key.clone()),
//                 ("to_key".to_string(), self.to_key.clone()),
//             }
//         }
//     }

//     impl FromBytes for UpdateClass {
//         fn from_bytes(bytes: &[u8]) -> MogResult<UpdateClass> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let key = try!(args.extract_key());
//             let class = try!(args.extract_required_string("class", MogError::NoClass));

//             Ok(UpdateClass {
//                 domain: domain,
//                 key: key,
//                 new_class: class,
//             })
//         }
//     }

//     impl ToArgs for UpdateClass {
//         fn to_args(&self) -> Vec<(String, String)> {
//             vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("key".to_string(), self.key.clone()),
//                 ("class".to_string(), self.new_class.clone()),
//             }
//         }
//     }

//     impl FromBytes for Delete {
//         fn from_bytes(bytes: &[u8]) -> MogResult<Delete> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let key = try!(args.extract_key());

//             Ok(Delete {
//                 domain: domain,
//                 key: key,
//             })
//         }
//     }

//     impl ToArgs for Delete {
//         fn to_args(&self) -> Vec<(String, String)> {
//             vec!{
//                 ("domain".to_string(), self.domain.clone()),
//                 ("key".to_string(), self.key.clone()),
//             }
//         }
//     }

//     impl FromBytes for ListKeys {
//         fn from_bytes(bytes: &[u8]) -> MogResult<ListKeys> {
//             let mut args = ArgsHash::from_bytes(bytes);
//             let domain = try!(args.extract_domain());
//             let prefix = args.extract_optional_string("prefix");
//             let limit = args.extract_optional_int("limit");
//             let after = args.extract_optional_string("after");

//             Ok(ListKeys {
//                 domain: domain,
//                 prefix: prefix,
//                 limit: limit,
//                 after: after,
//             })
//         }
//     }

//     impl ToArgs for ListKeys {
//         fn to_args(&self) -> Vec<(String, String)> {
//             let mut rv = vec!{
//                 ("domain".to_string(), self.domain.clone()),
//             };

//             if self.prefix.is_some() {
//                 rv.push(("prefix".to_string(), self.prefix.clone().unwrap()));
//             }

//             if self.after.is_some() {
//                 rv.push(("after".to_string(), self.after.clone().unwrap()));
//             }

//             if self.limit.is_some() {
//                 rv.push(("limit".to_string(), self.limit.clone().unwrap().to_string()));
//             }

//             rv
//         }
//     }

//     impl FromBytes for Noop {
//         fn from_bytes(_bytes: &[u8]) -> MogResult<Noop> {
//             Ok(Noop)
//         }
//     }

//     impl ToArgs for Noop {
//         fn to_args(&self) -> Vec<(String, String)> {
//             vec![]
//         }
//     }
// }
