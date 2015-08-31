use std::fmt::Debug;

pub trait Request: Debug {
    fn op(&self) -> &'static str;
}

pub mod types {
    use super::Request;
    use super::super::args_hash::ArgsHash;
    use super::super::util::FromBytes;
    use super::super::error::{MogError, MogResult};
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

    impl FromBytes for CreateDomain {
        fn from_bytes(bytes: &[u8]) -> MogResult<CreateDomain> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());

            Ok(CreateDomain {
                domain: domain,
            })
        }
    }

    impl FromBytes for CreateOpen {
        fn from_bytes(bytes: &[u8]) -> MogResult<CreateOpen> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let key = try!(args.extract_key());
            let multi_dest = args.extract_bool_value("multi_dest", false);
            let size = args.extract_optional_int("size");

            Ok(CreateOpen {
                domain: domain,
                key: key,
                multi_dest: multi_dest,
                size: size,
            })
        }
    }

    impl FromBytes for CreateClose {
        fn from_bytes(bytes: &[u8]) -> MogResult<CreateClose> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let key = try!(args.extract_key());
            let fid = try!(args.extract_required_int("fid", MogError::NoFid));
            let devid = try!(args.extract_required_int("devid", MogError::NoDevid));
            let path = try!(args.extract_required_url("path", MogError::NoPath));
            let checksum = args.extract_optional_string("checksum");

            Ok(CreateClose {
                domain: domain,
                key: key,
                fid: fid,
                devid: devid,
                path: path,
                checksum: checksum,
            })
        }
    }

    impl FromBytes for GetPaths {
        fn from_bytes(bytes: &[u8]) -> MogResult<GetPaths> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let key = try!(args.extract_key());
            let noverify = args.extract_bool_value("noverify", false);
            let pathcount = args.extract_optional_int("pathcount");

            Ok(GetPaths {
                domain: domain,
                key: key,
                noverify: noverify,
                pathcount: pathcount,
            })
        }
    }

    impl FromBytes for FileInfo {
        fn from_bytes(bytes: &[u8]) -> MogResult<FileInfo> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let key = try!(args.extract_key());

            Ok(FileInfo {
                domain: domain,
                key: key,
            })
        }
    }

    impl FromBytes for Rename {
        fn from_bytes(bytes: &[u8]) -> MogResult<Rename> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let from_key = try!(args.extract_required_string("from_key", MogError::NoKey));
            let to_key = try!(args.extract_required_string("to_key", MogError::NoKey));

            Ok(Rename {
                domain: domain,
                from_key: from_key,
                to_key: to_key,
            })
        }
    }

    impl FromBytes for UpdateClass {
        fn from_bytes(bytes: &[u8]) -> MogResult<UpdateClass> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let key = try!(args.extract_key());
            let class = try!(args.extract_required_string("class", MogError::NoClass));

            Ok(UpdateClass {
                domain: domain,
                key: key,
                new_class: class,
            })
        }
    }

    impl FromBytes for Delete {
        fn from_bytes(bytes: &[u8]) -> MogResult<Delete> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let key = try!(args.extract_key());

            Ok(Delete {
                domain: domain,
                key: key,
            })
        }
    }

    impl FromBytes for ListKeys {
        fn from_bytes(bytes: &[u8]) -> MogResult<ListKeys> {
            let mut args = ArgsHash::from_bytes(bytes);
            let domain = try!(args.extract_domain());
            let prefix = args.extract_optional_string("prefix");
            let limit = args.extract_optional_int("limit");
            let after = args.extract_optional_string("after");

            Ok(ListKeys {
                domain: domain,
                prefix: prefix,
                limit: limit,
                after: after,
            })
        }
    }

    impl FromBytes for Noop {
        fn from_bytes(_bytes: &[u8]) -> MogResult<Noop> {
            Ok(Noop)
        }
    }
}
