use std::collections::HashMap;
use std::str;
use super::error::{MogError, MogResult};
use url::{form_urlencoded, Url};

#[derive(Debug)]
pub enum Request {
    CreateDomain { domain: String },

    CreateOpen  { domain: String, key: String, multi_dest: bool, size: Option<u64> },
    CreateClose { domain: String, key: String, fid: u64, devid: u64, path: Url, checksum: Option<String> },
    GetPaths    { domain: String, key: String, noverify: bool, pathcount: Option<u64> },
    FileInfo    { domain: String, key: String },
    Rename      { domain: String, from_key: String, to_key: String },
    UpdateClass { domain: String, key: String, new_class: String },
    Delete      { domain: String, key: String },
    ListKeys    { domain: String, prefix: Option<String>, after: Option<String>, limit: Option<u64> },

    Noop,
}

impl Request {
    pub fn from_bytes(bytes: &[u8]) -> MogResult<Request> {
        let mut toks = bytes.split(|&b| b == b' ');
        let op = toks.next();
        let args = parse_urlencoded_args(toks.next());

        match op.map(|bs| str::from_utf8(bs)) {
            Some(Ok("create_domain")) => Request::create_domain_from_args(args),

            Some(Ok("create_open"))   => Request::create_open_from_args(args),
            Some(Ok("create_close"))  => Request::create_close_from_args(args),
            Some(Ok("get_paths"))     => Request::get_paths_from_args(args),
            Some(Ok("file_info"))     => Request::domain_and_key_request_from_args("file_info", args),
            Some(Ok("rename"))        => Request::rename_from_args(args),
            Some(Ok("updateclass"))   => Request::updateclass_from_args(args),
            Some(Ok("delete"))        => Request::domain_and_key_request_from_args("delete", args),
            Some(Ok("list_keys"))     => Request::list_keys_from_args(args),

            Some(Ok("noop"))          => Ok(Request::Noop),

            Some(Ok(""))     => Err(MogError::UnknownCommand(None)),
            Some(Ok(string)) => Err(MogError::UnknownCommand(Some(string.to_string()))),
            Some(Err(utf8e)) => Err(MogError::Utf8(utf8e)),
            None => Err(MogError::UnknownCommand(None)),
        }
    }

    pub fn op(&self) -> &str {
        use self::Request::*;

        match self {
            &CreateDomain {..} => "create_domain",
            &CreateOpen {..}   => "create_open",
            &CreateClose {..}  => "create_close",
            &GetPaths {..}     => "get_paths",
            &FileInfo {..}     => "file_info",
            &Rename {..}       => "rename",
            &UpdateClass {..}  => "updateclass",
            &Delete {..}       => "delete",
            &ListKeys {..}     => "list_keys",
            &Noop              => "noop",
        }
    }

    fn create_domain_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        match args_hash.get("domain") {
            Some(&domain) => Ok(Request::CreateDomain { domain: domain.to_string() }),
            None => Err(MogError::NoDomain),
        }
    }

    fn create_open_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        let multi_dest = coerce_to_bool(args_hash.get("multi_dest").map(|b| *b).unwrap_or("false"));
        let size = args_hash.get("size").and_then(|&s| u64::from_str_radix(s, 10).ok());

        match (args_hash.get("domain"), args_hash.get("key")) {
            (Some(&domain), Some(&key)) => Ok(Request::CreateOpen {
                domain: domain.to_string(),
                key: key.to_string(),
                multi_dest: multi_dest,
                size: size,
            }),
            (None, _) => Err(MogError::NoDomain),
            (_, None) => Err(MogError::NoKey),
        }
    }

    fn create_close_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        let domain = *try!(args_hash.get("domain").ok_or(MogError::NoDomain));
        let key = *try!(args_hash.get("key").ok_or(MogError::NoKey));
        let fid = try!(args_hash.get("fid").and_then(|f| u64::from_str_radix(f, 10).ok()).ok_or(MogError::NoFid));
        let devid = try!(args_hash.get("devid").and_then(|f| u64::from_str_radix(f, 10).ok()).ok_or(MogError::NoDevid));
        let path = try!(args_hash.get("path").and_then(|u| Url::parse(u).ok()).ok_or(MogError::NoPath));
        let checksum = args_hash.get("checksum").map(|c| c.to_string());

        Ok(Request::CreateClose {
            domain: domain.to_string(),
            key: key.to_string(),
            fid: fid,
            devid: devid,
            path: path,
            checksum: checksum,
        })
    }

    fn get_paths_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        let domain = *try!(args_hash.get("domain").ok_or(MogError::NoDomain));
        let key = *try!(args_hash.get("key").ok_or(MogError::NoKey));
        let noverify = coerce_to_bool(args_hash.get("noverify").map(|b| *b).unwrap_or("false"));
        let pathcount = args_hash.get("pathcount").and_then(|&s| u64::from_str_radix(s, 10).ok());

        Ok(Request::GetPaths {
            domain: domain.to_string(),
            key: key.to_string(),
            noverify: noverify,
            pathcount: pathcount,
        })
    }

    fn rename_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        let domain = *try!(args_hash.get("domain").ok_or(MogError::NoDomain));
        let from_key = *try!(args_hash.get("from_key").ok_or(MogError::NoKey));
        let to_key = *try!(args_hash.get("to_key").ok_or(MogError::NoKey));

        Ok(Request::Rename {
            domain: domain.to_string(),
            from_key: from_key.to_string(),
            to_key: to_key.to_string(),
        })
    }

    fn updateclass_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        let domain = *try!(args_hash.get("domain").ok_or(MogError::NoDomain));
        let key = *try!(args_hash.get("key").ok_or(MogError::NoKey));
        let class = *try!(args_hash.get("class").ok_or(MogError::NoClass));

        Ok(Request::UpdateClass {
            domain: domain.to_string(),
            key: key.to_string(),
            new_class: class.to_string(),
        })
    }

    fn list_keys_from_args(args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        let domain = try!(args_hash.get("domain").ok_or(MogError::NoDomain));
        let prefix = args_hash.get("prefix").map(|c| c.to_string());
        let limit = args_hash.get("limit").and_then(|&s| u64::from_str_radix(s, 10).ok());
        let after = args_hash.get("after").map(|c| c.to_string());

        Ok(Request::ListKeys {
            domain: domain.to_string(),
            prefix: prefix,
            limit: limit,
            after: after,
        })
    }

    fn domain_and_key_request_from_args(op: &str, args: Vec<(String, String)>) -> MogResult<Request> {
        let args_hash = args_to_hash(&args);
        match (op, args_hash.get("domain"), args_hash.get("key")) {
            ("file_info",   Some(&domain), Some(&key)) => Ok(Request::FileInfo   { domain: domain.to_string(), key: key.to_string() }),
            ("delete",      Some(&domain), Some(&key)) => Ok(Request::Delete     { domain: domain.to_string(), key: key.to_string() }),
            (_, None, _) => Err(MogError::NoDomain),
            (_, _, None) => Err(MogError::NoKey),
            _ => Err(MogError::UnknownCommand(Some(op.to_string()))),
        }
    }
}

fn parse_urlencoded_args(args: Option<&[u8]>) -> Vec<(String, String)> {
    form_urlencoded::parse(args.unwrap_or(&[]))
}

fn args_to_hash(args: &[(String, String)]) -> HashMap<&str, &str> {
    let mut rv = HashMap::new();
    for &(ref k, ref v) in args.iter() {
        rv.entry(k.as_ref()).or_insert(v.as_ref());
    }
    rv
}

fn coerce_to_bool(string: &str) -> bool {
    match string.to_lowercase().as_ref() {
        "true" | "t" | "1" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::MogError;
    use super::*;

    #[test]
    fn request_from_no_bytes() {
        assert!(matches!(Request::from_bytes(b""),
                         Err(MogError::UnknownCommand(None))));
    }

    #[test]
    fn unknown_command() {
        let request = Request::from_bytes(b"this_command_doesnt_exist");

        match request {
            Err(MogError::UnknownCommand(Some(ref s))) => {
                assert_eq!("this_command_doesnt_exist", s);
            },
            _ => panic!("Bad request parse: request = {:?}", request),
        }
    }

    #[test]
    fn known_command() {
        let request = Request::from_bytes(b"file_info domain=test_domain&key=test_key");
        match request {
            Ok(Request::FileInfo { ref domain, ref key }) => {
                assert_eq!("test_domain", domain);
                assert_eq!("test_key", key);
            },
            _ => panic!("Bad request parse: request = {:?}", request),
        }
    }

    #[test]
    fn request_with_no_args() {
        let request = Request::from_bytes(b"create_open");
        match request {
            Err(MogError::NoDomain) => {},
            _ => panic!("Bad request parse: request = {:?}", request),
        }
    }
}
