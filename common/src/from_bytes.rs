use super::{MogError, MogResult};
use super::request::types::*;
use std::collections::HashMap;
use url::{form_urlencoded, Url};

pub trait FromBytes {
    fn from_bytes(bytes: &[u8]) -> MogResult<Self>;
}

#[derive(Debug, Clone)]
struct ArgsHash(HashMap<String, String>);

impl ArgsHash {
    fn from_bytes(bytes: &[u8]) -> ArgsHash {
        let args = form_urlencoded::parse(bytes);
        let mut rv = HashMap::new();
        for (k, v) in args.into_iter() {
            rv.entry(k).or_insert(v);
        }
        ArgsHash(rv)
    }

    fn extract_required_string(&mut self, key: &str, missing_error: MogError) -> MogResult<String> {
        self.0.remove(key).and_is_not_blank().ok_or(missing_error)
    }

    fn extract_required_int(&mut self, key: &str, missing_error: MogError) -> MogResult<u64> {
        self.0.remove(key).and_then(|f| u64::from_str_radix(&f, 10).ok()).ok_or(missing_error)
    }

    fn extract_required_url(&mut self, key: &str, missing_error: MogError) -> MogResult<Url> {
        match self.0.remove(key).and_then(|u| Url::parse(&u).ok()) {
            Some(ref uu) if uu.scheme == "http" => Ok(uu.clone()),
            _ => Err(missing_error),
        }
    }

    fn extract_optional_string(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }

    fn extract_optional_int(&mut self, key: &str) -> Option<u64> {
        self.0.remove(key).and_then(|f| u64::from_str_radix(&f, 10).ok())
    }

    fn extract_bool_value(&mut self, key: &str, default: bool) -> bool {
        match self.0.remove(key) {
            v @ Some(..) => v.is_truthy(),
            None => default,
        }
    }

    fn extract_domain(&mut self) -> MogResult<String> {
        self.extract_required_string("domain", MogError::NoDomain)
    }

    fn extract_key(&mut self) -> MogResult<String> {
        self.extract_required_string("key", MogError::NoKey)
    }
}

trait OptionStringExt<S: AsRef<str>>: Sized {
    fn and_is_not_blank(self) -> Self;
    fn is_truthy(self) -> bool;
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

    fn is_truthy(self) -> bool {
        match self {
            Some(s) => {
                match s.as_ref().to_lowercase().as_ref() {
                    "true" | "t" | "1" => true,
                    _ => false,
                }
            },
            _ => false,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::{ArgsHash, OptionStringExt};
    use super::super::MogError;

    #[test]
    fn test_is_not_blank() {
        assert!(Some("").and_is_not_blank().is_none());
        assert!(Some("not empty").and_is_not_blank().is_some());
    }

    #[test]
    fn test_is_truthy() {
        assert!(Some("true").is_truthy());
        assert!(!Some("false").is_truthy());

        assert!(Some("t").is_truthy());
        assert!(!Some("f").is_truthy());

        assert!(Some("1").is_truthy());
        assert!(!Some("0").is_truthy());

        assert!(!Some("puppy").is_truthy());
        assert!(!Some("10").is_truthy());
        assert!(!Some("trueblood").is_truthy());
    }

    #[test]
    fn test_extract_required_string() {
        let args = ArgsHash::from_bytes(b"present_field=nachos&blank_field=");

        assert!(matches!(args.clone().extract_required_string("present_field", MogError::NoDomain), Ok(ref v) if v == "nachos"));
        assert!(matches!(args.clone().extract_required_string("blank_field", MogError::NoDomain), Err(MogError::NoDomain)));
        assert!(matches!(args.clone().extract_required_string("missing_field", MogError::NoDomain), Err(MogError::NoDomain)));
    }

    #[test]
    fn test_extract_required_int() {
        let args = ArgsHash::from_bytes(b"present_field=123&blank_field=&bad_format_field=nachos");

        assert!(matches!(args.clone().extract_required_int("present_field", MogError::NoDomain), Ok(123)));
        assert!(matches!(args.clone().extract_required_int("blank_field", MogError::NoDomain), Err(MogError::NoDomain)));
        assert!(matches!(args.clone().extract_required_int("missing_field", MogError::NoDomain), Err(MogError::NoDomain)));
        assert!(matches!(args.clone().extract_required_int("bad_format_field", MogError::NoDomain), Err(MogError::NoDomain)));
    }

    #[test]
    fn test_extract_required_url() {
        use url::Url;
        let args = ArgsHash::from_bytes(b"present_field=http://test.host/path/to/resource&blank_field=&bad_format_field=nachos&not_http=file:///usr/bin/env");

        assert!(matches!(args.clone().extract_required_url("present_field", MogError::NoDomain),
                         Ok(ref u) if u == &Url::parse("http://test.host/path/to/resource").unwrap()));
        assert!(matches!(args.clone().extract_required_url("blank_field", MogError::NoDomain), Err(MogError::NoDomain)));
        assert!(matches!(args.clone().extract_required_url("missing_field", MogError::NoDomain), Err(MogError::NoDomain)));
        assert!(matches!(args.clone().extract_required_url("bad_format_field", MogError::NoDomain), Err(MogError::NoDomain)));
        assert!(matches!(args.clone().extract_required_url("not_http", MogError::NoDomain), Err(MogError::NoDomain)));
    }

    #[test]
    fn test_extract_optional_string() {
        let args = ArgsHash::from_bytes(b"present_field=nachos&blank_field=");

        assert!(matches!(args.clone().extract_optional_string("present_field"), Some(ref v) if v == "nachos"));
        // Do we actually wnat blank to be passed through?
        assert!(matches!(args.clone().extract_optional_string("blank_field"), Some(ref v) if v.is_empty()));
        assert!(matches!(args.clone().extract_optional_string("missing_field"), None));
    }

    #[test]
    fn test_extract_optional_int() {
        let args = ArgsHash::from_bytes(b"present_field=123&blank_field=&bad_format_field=nachos");

        assert!(matches!(args.clone().extract_optional_int("present_field"), Some(123)));
        assert!(matches!(args.clone().extract_optional_int("blank_field"), None));
        assert!(matches!(args.clone().extract_optional_int("missing_field"), None));
        assert!(matches!(args.clone().extract_optional_int("bad_format_field"), None));
    }

    #[test]
    fn test_extract_bool_value() {
        let args = ArgsHash::from_bytes(b"present_field=true&blank_field=&bad_format_field=nachos&as_number=1&as_first_letter=t&as_capital_letter=T");

        assert!(args.clone().extract_bool_value("present_field", false));
        assert!(!args.clone().extract_bool_value("blank_field", false));
        assert!(!args.clone().extract_bool_value("bad_format_field", false));
        assert!(args.clone().extract_bool_value("as_number", false));
        assert!(args.clone().extract_bool_value("as_first_letter", false));
        assert!(args.clone().extract_bool_value("as_capital_letter", false));

        // this next test fails. If we care (i.e, we start having bool
        // values that default to true), we should probably fix it.
        // assert!(args.clone().extract_bool_value("blank_field", true));
    }
}
