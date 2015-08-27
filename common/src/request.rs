use std::fmt::Debug;

pub trait Request: Debug {
    fn op(&self) -> &'static str;
}

pub mod types {
    use super::Request;
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
            // impl ToArgs for $name {
            //     fn args(&self) -> Vec<(String, String)> {
            //         vec![]
            //     }
            // }
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
