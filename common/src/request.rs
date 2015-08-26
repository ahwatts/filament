use std::collections::HashMap;
use std::fmt::Debug;
use super::error::MogResult;

pub trait Request: Debug + ToArgs {
    fn op(&self) -> &'static str;
}

pub fn boxed_result<T: Request + 'static>(req: MogResult<T>) -> MogResult<Box<Request>> {
    req.map(|r| Box::new(r) as Box<Request>)
}

pub mod types {
    use super::{Request, ToArgs};
    use url::Url;

    macro_rules! request_type {
        ( $name:ident $op:expr ) => {
            #[derive(Debug)]
            pub struct $name;
            impl Request for $name {
                fn op(&self) -> &'static str {
                    $op
                }
            }
            impl ToArgs for $name {
                fn args(&self) -> Vec<(String, String)> {
                    vec![]
                }
            }
        };
        ( $name:ident $op:expr { $( $f:ident : $t:ty ),* } ) => {
            #[derive(Debug)]
            pub struct $name {
                $( pub $f: $t, )*
            }
            impl Request for $name {
                fn op(&self) -> &'static str {
                    $op
                }
            }
        };
    }

    request_type!{ CreateDomain "create_domain" { domain: String } }
    request_type!{ CreateOpen   "create_open"   { domain: String, key: String, multi_dest: bool, size: Option<u64> } }
    request_type!{ CreateClose  "create_close"  { domain: String, key: String, fid: u64, devid: u64, path: Url, checksum: Option<String> } }
    request_type!{ GetPaths     "get_paths"     { domain: String, key: String, noverify: bool, pathcount: Option<u64> } }
    request_type!{ FileInfo     "file_info"     { domain: String, key: String } }
    request_type!{ Rename       "rename"        { domain: String, from_key: String, to_key: String } }
    request_type!{ UpdateClass  "updateclass"   { domain: String, key: String, new_class: String } }
    request_type!{ Delete       "delete"        { domain: String, key: String } }
    request_type!{ ListKeys     "list_keys"     { domain: String, prefix: Option<String>, after: Option<String>, limit: Option<u64> } }
    request_type!{ Noop         "noop" }
}

pub trait ToArgs {
    fn args(&self) -> Vec<(String, String)>;

    fn args_hash(&self) -> HashMap<String, String> {
        let mut rv = HashMap::new();
        for (k, v) in self.args().into_iter() {
            rv.entry(k).or_insert(v);
        }
        rv
    }
}

impl ToArgs for types::CreateDomain {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
        }
    }
}

impl ToArgs for types::CreateOpen {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("multi_dest".to_string(), self.multi_dest.to_string()),
        };

        if self.size.is_some() {
            rv.push(("size".to_string(), self.size.clone().unwrap().to_string()));
        }

        rv
    }
}

impl ToArgs for types::CreateClose {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("fid".to_string(), self.fid.to_string()),
            ("devid".to_string(), self.devid.to_string()),
            ("path".to_string(), self.path.to_string()),
        };

        if self.checksum.is_some() {
            rv.push(("checksum".to_string(), self.checksum.clone().unwrap()));
        }

        rv
    }
}

impl ToArgs for types::GetPaths {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("noverify".to_string(), self.noverify.to_string()),
        };

        if self.pathcount.is_some() {
            rv.push(("pathcount".to_string(), self.pathcount.clone().unwrap().to_string()));
        }

        rv
    }
}

impl ToArgs for types::FileInfo {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
        }
    }
}

impl ToArgs for types::Rename {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("from_key".to_string(), self.from_key.clone()),
            ("to_key".to_string(), self.to_key.clone()),
        }
    }
}

impl ToArgs for types::UpdateClass {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("class".to_string(), self.new_class.clone()),
        }
    }
}

impl ToArgs for types::Delete {
    fn args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
        }
    }
}

impl ToArgs for types::ListKeys {
    fn args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
        };

        if self.prefix.is_some() {
            rv.push(("prefix".to_string(), self.prefix.clone().unwrap()));
        }

        if self.after.is_some() {
            rv.push(("after".to_string(), self.after.clone().unwrap()));
        }

        if self.limit.is_some() {
            rv.push(("limit".to_string(), self.limit.clone().unwrap().to_string()));
        }

        rv
    }
}

// #[cfg(test)]
// mod tests {
//     use super::super::MogError;
//     use super::*;

//     #[test]
//     fn request_from_no_bytes() {
//         assert!(matches!(Request::from_bytes(b""),
//                          Err(MogError::UnknownCommand(None))));
//     }

//     #[test]
//     fn unknown_command() {
//         let request = Request::from_bytes(b"this_command_doesnt_exist");

//         match request {
//             Err(MogError::UnknownCommand(Some(ref s))) => {
//                 assert_eq!("this_command_doesnt_exist", s);
//             },
//             _ => panic!("Bad request parse: request = {:?}", request),
//         }
//     }

//     #[test]
//     fn known_command() {
//         let request = Request::from_bytes(b"file_info domain=test_domain&key=test_key");
//         match request {
//             Ok(Request::FileInfo { ref domain, ref key }) => {
//                 assert_eq!("test_domain", domain);
//                 assert_eq!("test_key", key);
//             },
//             _ => panic!("Bad request parse: request = {:?}", request),
//         }
//     }

//     #[test]
//     fn request_with_no_args() {
//         let request = Request::from_bytes(b"create_open");
//         match request {
//             Err(MogError::NoDomain) => {},
//             _ => panic!("Bad request parse: request = {:?}", request),
//         }
//     }
// }
