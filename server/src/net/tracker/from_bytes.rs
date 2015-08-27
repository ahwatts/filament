use mogilefs_common::requests::*;
use mogilefs_common::{MogError, MogResult};
use std::collections::HashMap;
use url::{form_urlencoded, Url};

pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> MogResult<Self>;
}

impl FromBytes for CreateDomain {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateDomain> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));

        Ok(CreateDomain {
            domain: domain,
        })
    }
}

impl FromBytes for CreateOpen {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateOpen> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").and_is_not_blank().ok_or(MogError::NoKey));
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
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").and_is_not_blank().ok_or(MogError::NoKey));
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
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").and_is_not_blank().ok_or(MogError::NoKey));
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
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").and_is_not_blank().ok_or(MogError::NoKey));

        Ok(FileInfo {
            domain: domain,
            key: key,
        })
    }
}

impl FromBytes for Rename {
    fn from_bytes(bytes: &[u8]) -> MogResult<Rename> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let from_key = try!(args.remove("from_key").and_is_not_blank().ok_or(MogError::NoKey));
        let to_key = try!(args.remove("to_key").and_is_not_blank().ok_or(MogError::NoKey));

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
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").and_is_not_blank().ok_or(MogError::NoKey));
        let class = try!(args.remove("class").and_is_not_blank().ok_or(MogError::NoClass));

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
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
        let key = try!(args.remove("key").and_is_not_blank().ok_or(MogError::NoKey));

        Ok(Delete {
            domain: domain,
            key: key,
        })
    }
}

impl FromBytes for ListKeys {
    fn from_bytes(bytes: &[u8]) -> MogResult<ListKeys> {
        let mut args = bytes_to_args_hash(bytes);
        let domain = try!(args.remove("domain").and_is_not_blank().ok_or(MogError::NoDomain));
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

trait OptionStringExt<S: AsRef<str>>: Sized {
    fn and_is_not_blank(self) -> Self;
}

impl<S: AsRef<str>> OptionStringExt<S> for Option<S> {
    fn and_is_not_blank(self) -> Option<S> {
        self.and_then(|s| {
            match s.as_ref().is_empty() {
                true => None,
                false => Some(s),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mogilefs_common::MogError;
    use mogilefs_common::requests::*;
    use url::Url;

    macro_rules! assert_eq_2 {
        ( $expected:expr, $actual:expr ) => {
            assert!($expected == $actual, "{:?} was not {:?}", stringify!($actual), $expected);
        }
    }

    #[test]
    fn test_coerce_to_bool() {
        assert_eq_2!(true, super::coerce_to_bool("true"));
        assert_eq_2!(false, super::coerce_to_bool("false"));

        assert_eq_2!(true, super::coerce_to_bool("t"));
        assert_eq_2!(false, super::coerce_to_bool("f"));

        assert_eq_2!(true, super::coerce_to_bool("1"));
        assert_eq_2!(false, super::coerce_to_bool("0"));

        assert_eq_2!(false, super::coerce_to_bool("puppy"));
        assert_eq_2!(false, super::coerce_to_bool("10"));
        assert_eq_2!(false, super::coerce_to_bool("trueblood"));
    }

    macro_rules! matches_request {
        ( $name: expr, $req:expr, $req_match:pat => $extra:block ) => {
            match $req {
                $req_match => $extra,
                r @ _ => panic!("Bad request parse for {:?}: parsed request = {:?}", $name, r),
            }
        }
    }

    #[test]
    fn create_domain_from_bytes() {
        matches_request!{
            "Happy path",
            CreateDomain::from_bytes(b"domain=test_domain"),
            Ok(CreateDomain { ref domain }) => {
                assert_eq!("test_domain", domain);
            }
        }

        matches_request!{
            "Empty byte string",
            CreateDomain::from_bytes(&[]),
            Err(MogError::NoDomain) => {}
        }

        matches_request!{
            "Blank domain",
            CreateDomain::from_bytes(b"domain="),
            Err(MogError::NoDomain) => {}
        }
    }

    #[test]
    fn create_open_from_bytes() {
        matches_request!{
            "No optional params",
            CreateOpen::from_bytes(b"domain=test_domain&key=test/key/1"),
            Ok(req @ CreateOpen {..}) => {
                assert_eq!("test_domain", req.domain);
                assert_eq!("test/key/1", req.key);
                assert_eq!(false, req.multi_dest);
                assert_eq!(None, req.size);
            }
        }

        matches_request!{
            "With multi_dest",
            CreateOpen::from_bytes(b"domain=test_domain&key=test/key/1&multi_dest=1"),
            Ok(req @ CreateOpen {..}) => {
                assert_eq!("test_domain", req.domain);
                assert_eq!("test/key/1", req.key);
                assert_eq!(true, req.multi_dest);
                assert_eq!(None, req.size);
            }
        }

        matches_request!{
            "With size",
            CreateOpen::from_bytes(b"domain=test_domain&key=test/key/1&size=12"),
            Ok(req @ CreateOpen {..}) => {
                assert_eq!("test_domain", req.domain);
                assert_eq!("test/key/1", req.key);
                assert_eq!(false, req.multi_dest);
                assert_eq!(Some(12), req.size);
            }
        }

        matches_request!{
            "Blank domain",
            CreateOpen::from_bytes(b"domain=&key=test/key/1"),
            Err(MogError::NoDomain) => {}
        }

        matches_request!{
            "Blank key",
            CreateOpen::from_bytes(b"domain=test_domain&key="),
            Err(MogError::NoKey) => {}
        }
    }

    #[test]
    fn create_close_from_bytes() {
        matches_request!{
            "No optional params",
            CreateClose::from_bytes(b"domain=test_domain&key=test/key/1&fid=25&devid=2&path=http://test.storage.host/dev2/0/0/0000025.fid"),
            Ok(req @ CreateClose {..}) => {
                assert_eq!("test_domain", req.domain);
                assert_eq!("test/key/1", req.key);
                assert_eq!(25, req.fid);
                assert_eq!(2, req.devid);
                assert_eq!(Url::parse("http://test.storage.host/dev2/0/0/0000025.fid").unwrap(), req.path);
            }
        }
    }
}
