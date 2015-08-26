use mogilefs_common::request::types::*;
use mogilefs_common::{MogError, MogResult};
use std::collections::HashMap;
use url::{form_urlencoded, Url};

pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> MogResult<Self>;
}

impl FromBytes for CreateDomain {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateDomain> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));

        Ok(CreateDomain {
            domain: domain,
        })
    }
}

impl FromBytes for CreateOpen {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateOpen> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").ok_or(MogError::NoKey));
        let multi_dest = coerce_to_bool(&args.remove("multi_dest").unwrap_or("false".to_string()));
        let size = args.remove("size").and_then(|s| u64::from_str_radix(&s, 10).ok());

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
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").ok_or(MogError::NoKey));
        let fid = try!(args.remove("fid").and_then(|f| u64::from_str_radix(&f, 10).ok()).ok_or(MogError::NoFid));
        let devid = try!(args.remove("devid").and_then(|f| u64::from_str_radix(&f, 10).ok()).ok_or(MogError::NoDevid));
        let path = try!(args.remove("path").and_then(|u| Url::parse(&u).ok()).ok_or(MogError::NoPath));
        let checksum = args.remove("checksum");

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
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").ok_or(MogError::NoKey));
        let noverify = coerce_to_bool(&args.remove("noverify").unwrap_or("false".to_string()));
        let pathcount = args.remove("pathcount").and_then(|s| u64::from_str_radix(&s, 10).ok());

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
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").ok_or(MogError::NoKey));

        Ok(FileInfo {
            domain: domain,
            key: key,
        })
    }
}

impl FromBytes for Rename {
    fn from_bytes(bytes: &[u8]) -> MogResult<Rename> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let from_key = try!(args.remove("from_key").ok_or(MogError::NoKey));
        let to_key = try!(args.remove("to_key").ok_or(MogError::NoKey));

        Ok(Rename {
            domain: domain,
            from_key: from_key,
            to_key: to_key,
        })
    }
}

impl FromBytes for UpdateClass {
    fn from_bytes(bytes: &[u8]) -> MogResult<UpdateClass> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").ok_or(MogError::NoKey));
        let class = try!(args.remove("class").ok_or(MogError::NoClass));

        Ok(UpdateClass {
            domain: domain,
            key: key,
            new_class: class,
        })
    }
}

impl FromBytes for Delete {
    fn from_bytes(bytes: &[u8]) -> MogResult<Delete> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").ok_or(MogError::NoKey));

        Ok(Delete {
            domain: domain,
            key: key,
        })
    }
}

impl FromBytes for ListKeys {
    fn from_bytes(bytes: &[u8]) -> MogResult<ListKeys> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").ok_or(MogError::NoDomain));
        let prefix = args.remove("prefix");
        let limit = args.remove("limit").and_then(|s| u64::from_str_radix(&s, 10).ok());
        let after = args.remove("after");

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

fn bytes_to_args_hash(bytes: &[u8]) -> HashMap<String, String> {
    let args = form_urlencoded::parse(bytes);
    let mut rv = HashMap::new();
    for (k, v) in args.into_iter() {
        rv.entry(k).or_insert(v);
    }
    rv
}

fn coerce_to_bool(string: &str) -> bool {
    match string.to_lowercase().as_ref() {
        "true" | "t" | "1" => true,
        _ => false,
    }
}
