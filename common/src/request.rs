//! Request (and response) traits and types.

use std::any::Any;
use std::fmt::Debug;
use std::str;
use super::args_hash::ArgsHash;
use super::backend::{Backend};
use super::error::{MogError, MogResult};
use super::util::{FromBytes, ToArgs, ToUrlencodedString};
use url::Url;

/// A tracker request.
pub trait Request: Debug + ToArgs + Sync + Send {
    /// Return the "op code", or the first bit before the query
    /// string, for this request type.
    fn op(&self) -> &'static str;

    /// Construct the appropriate response type for this request. This
    /// method shouldn't need to use the receiver `self`, but it is
    /// included to make the trait object-safe.
    fn response_from_bytes(&self, &[u8]) -> MogResult<Response>;

    /// Perform this request's action on the `Backend`. Ultimately
    /// forwards `self` on to one of the methods in `Backend`.
    fn perform(&self, &Backend) -> MogResult<Response>;
}

impl<R: Request + ?Sized> Request for Box<R> {
    fn op(&self) -> &'static str { (**self).op() }

    fn response_from_bytes(&self, bytes: &[u8]) -> MogResult<Response> {
        (**self).response_from_bytes(bytes)
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        (**self).perform(backend)
    }
}

impl FromBytes for Box<Request> {
    fn from_bytes(bytes: &[u8]) -> MogResult<Box<Request>> {
        let mut toks = bytes.split(|&b| b == b' ');
        let op = toks.next();
        let args = toks.next().unwrap_or(&[]);

        match op.map(|bs| str::from_utf8(bs)) {
            Some(Ok("create_domain")) => CreateDomain::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("create_open"))   => CreateOpen::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("create_close"))  => CreateClose::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("file_info"))     => FileInfo::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("get_paths"))     => GetPaths::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("rename"))        => Rename::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("updateclass"))   => UpdateClass::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("delete"))        => Delete::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("list_keys"))     => ListKeys::from_bytes(args).map(|r| Box::new(r) as Box<Request>),
            Some(Ok("noop"))          => Noop::from_bytes(args).map(|r| Box::new(r) as Box<Request>),

            Some(Ok(""))     => Err(MogError::UnknownCommand(None)),
            Some(Ok(string)) => Err(MogError::UnknownCommand(Some(string.to_string()))),
            Some(Err(utf8e)) => Err(MogError::Utf8(utf8e)),
            None => Err(MogError::UnknownCommand(None)),
        }
    }
}

/// The response to a tracker request.
#[derive(Debug, PartialEq, Eq)]
pub enum Response {
    Empty,
    CreateDomain(CreateDomain),
    CreateOpen(CreateOpenResponse),
    FileInfo(FileInfoResponse),
    GetPaths(GetPathsResponse),
    ListKeys(ListKeysResponse),
}

impl Response {
    pub fn downcast<T: Any>(self) -> Option<T> {
        use self::Response::*;

        match self {
            Empty           => downcast(()),
            CreateDomain(r) => downcast(r),
            CreateOpen(r)   => downcast(r),
            FileInfo(r)     => downcast(r),
            GetPaths(r)     => downcast(r),
            ListKeys(r)     => downcast(r),
        }
    }
}

impl ToArgs for Response {
    fn to_args(&self) -> Vec<(String, String)> {
        use self::Response::*;
        match self {
            &Empty               => vec![],
            &CreateDomain(ref r) => r.to_args(),
            &CreateOpen(ref r)   => r.to_args(),
            &FileInfo(ref r)     => r.to_args(),
            &GetPaths(ref r)     => r.to_args(),
            &ListKeys(ref r)     => r.to_args(),
        }
    }
}

fn downcast<F: Any, T: Any>(thing: F) -> Option<T> {
    // Can't move v out of borrowed context...
    // (&thing as &Any).downcast_ref::<T>().map(|v| *v)

    // It seems really expensive to move something out to the heap and
    // back just to cast it to a different type, but the alternative
    // is to downcast a ref and clone it...
    let any_thing: Box<Any> = Box::new(thing);
    match any_thing.downcast::<T>() {
        Ok(boxed) => Some(*boxed),
        Err(_) => {
            // This case returns you the Box<Any> that you passed in
            // to downcast. Since I obviously can't convert it in to a
            // T, I guess we can just lose the object to the ether?
            None
        },
    }
}

/// Something which can be coerced in to a `Response`.
pub trait ToResponse {
    fn to_response(self) -> Response;
}

impl ToResponse for () {
    fn to_response(self) -> Response {
        Response::Empty
    }
}

/// Something which can be rendered to a string for the MogileFS
/// tracker's line-based protocol.
pub trait Renderable {
    fn render(&self) -> String;
}

impl Renderable for Response {
    fn render(&self) -> String {
        format!("OK {}", self.to_urlencoded_string())
    }
}

/// A `create_domain` request.
///
/// Serves as its own response type. Looks like this:
///
/// ```text
/// request = "create_domain domain=test_domain_2\r\n"
/// response = "OK domain=test_domain_2\r\n"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDomain {
    pub domain: String,
}

impl Request for CreateDomain {
    fn op(&self) -> &'static str { "create_domain" }

    fn response_from_bytes(&self, bytes: &[u8]) -> MogResult<Response> {
        CreateDomain::from_bytes(bytes).map(|r| r.to_response())
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.create_domain(self).map(|r| r.to_response())
    }
}

// impl Response for CreateDomain {}

impl ToResponse for CreateDomain {
    fn to_response(self) -> Response {
        Response::CreateDomain(self)
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

impl ToArgs for CreateDomain {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
        }
    }
}

/// A `create_open` request.
///
/// Looks like this:
///
/// ```text
/// request = "create_open key=test/key/1&multi_dest=1&domain=test_domain_2\r\n"
/// response = "OK devid_1=1&path_1=http://127.0.0.1:7500/dev1/0/000/001/0000001927.fid&dev_count=1&fid=1927\r\n"
/// ```
#[derive(Debug, Clone)]
pub struct CreateOpen {
    pub domain: String,
    pub class: Option<String>,
    pub key: String,
    pub multi_dest: bool,
    pub size: Option<u64>
}

impl Request for CreateOpen {
    fn op(&self) -> &'static str { "create_open" }

    fn response_from_bytes(&self, bytes: &[u8]) -> MogResult<Response> {
        CreateOpenResponse::from_bytes(bytes).map(|r| r.to_response())
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.create_open(self).map(|r| r.to_response())
    }
}

impl FromBytes for CreateOpen {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateOpen> {
        let mut args = ArgsHash::from_bytes(bytes);
        let domain = try!(args.extract_domain());
        let class = args.extract_optional_string("class");
        let key = try!(args.extract_key());
        let multi_dest = args.extract_bool_value("multi_dest", false);
        let size = args.extract_optional_int("size");

        Ok(CreateOpen {
            domain: domain,
            class: class,
            key: key,
            multi_dest: multi_dest,
            size: size,
        })
    }
}

impl ToArgs for CreateOpen {
    fn to_args(&self) -> Vec<(String, String)> {
        let mut rv = vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("multi_dest".to_string(), self.multi_dest.to_string()),
        };

        if let Some(ref class) = self.class {
            rv.push(("class".to_string(), class.clone()));
        }

        if let Some(ref size) = self.size {
            rv.push(("size".to_string(), size.to_string()));
        }

        rv
    }
}

/// The response to a `create_open` request.
///
/// Looks like this:
///
/// ```text
/// request = "create_open key=test/key/1&multi_dest=1&domain=test_domain_2\r\n"
/// response = "OK devid_1=1&path_1=http://127.0.0.1:7500/dev1/0/000/001/0000001927.fid&dev_count=1&fid=1927\r\n"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOpenResponse {
    pub fid: u64,
    pub paths: Vec<(u64, Url)>,
}

// impl Response for CreateOpenResponse {}

impl ToResponse for CreateOpenResponse {
    fn to_response(self) -> Response {
        Response::CreateOpen(self)
    }
}

impl ToArgs for CreateOpenResponse {
    fn to_args(&self) -> Vec<(String, String)> {
        let mut args = vec!{
            ("fid".to_string(), self.fid.to_string()),
            ("dev_count".to_string(), self.paths.len().to_string()),
        };

        for (i, &(ref devid, ref url)) in self.paths.iter().enumerate() {
            args.push((format!("devid_{}", i + 1), devid.to_string()));
            args.push((format!("path_{}", i + 1), url.to_string()));
        }

        args
    }
}

impl FromBytes for CreateOpenResponse {
    fn from_bytes(bytes: &[u8]) -> MogResult<CreateOpenResponse> {
        let mut args = ArgsHash::from_bytes(bytes);
        let fid = try!(args.extract_required_int("fid", MogError::NoFid));
        let devcount = try!(args.extract_required_int("dev_count", MogError::Other("No device count".to_string(), None)));
        let mut paths = Vec::new();

        for i in (1..(devcount + 1)) {
            let devid = try!(args.extract_required_int(&format!("devid_{}", i), MogError::NoDevid));
            let url = try!(args.extract_required_url(&format!("path_{}", i), MogError::NoPath));
            paths.push((devid, url));
        }

        Ok(CreateOpenResponse {
            fid: fid,
            paths: paths,
        })
    }
}

/// A `create_close` request.
///
/// Looks like this:
///
/// ```text
/// request = "create_close fid=1927&key=test/key/1&domain=test_domain_2&devid=1&path=http://127.0.0.1:7500/dev1/0/000/001/0000001927.fid&size=4\r\n"
/// response = "OK \r\n"
/// ```
#[derive(Debug, Clone)]
pub struct CreateClose {
    pub domain: String,
    pub key: String,
    pub fid: u64,
    pub devid: u64,
    pub path: Url,
    pub checksum: Option<String>
}

impl Request for CreateClose {
    fn op(&self) -> &'static str { "create_close" }

    fn response_from_bytes(&self, _bytes: &[u8]) -> MogResult<Response> {
        Ok(Response::Empty)
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.create_close(self).map(|r| r.to_response())
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

impl ToArgs for CreateClose {
    fn to_args(&self) -> Vec<(String, String)> {
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

/// A `create_class` request.
///
/// Looks like this:
///
/// ```text
/// request = "create_class domain=rn_development_public&class=filament&replpolicy=MultipleHosts%28%29&mindevcount=1\r\n"
/// response = "OK domain=rn_development_public&class=filament&mindevcount=1\r\n"
/// ```
#[derive(Debug, Clone)]
pub struct CreateClass {
    pub domain: String,
    pub class: String,
    pub mindevcount: u64,
    pub replpolicy: Option<String>,
    pub hashtype: Option<String>,
    pub update: bool,
}

impl Request for CreateClass {
    fn op(&self) -> &'static str {
        "create_class"
    }

    fn response_from_bytes(&self, _bytes: &[u8]) -> MogResult<Response> {
        unimplemented!()
    }

    fn perform(&self, _backend: &Backend) -> MogResult<Response> {
        unimplemented!()
    }
}

impl FromBytes for CreateClass {
    fn from_bytes(_bytes: &[u8]) -> MogResult<CreateClass> {
        unimplemented!()
    }
}

impl ToArgs for CreateClass {
    fn to_args(&self) -> Vec<(String, String)> {
        unimplemented!()
    }
}

/// A `get_paths` request.
///
/// Looks like this:
///
/// ```text
/// request = "get_paths domain=test_domain_2&key=test/key/1&noverify=1&zone=\r\n"
/// response = "OK paths=1&path1=http://127.0.0.1:7500/dev1/0/000/001/0000001927.fid\r\n"
/// ```
#[derive(Debug, Clone)]
pub struct GetPaths {
    pub domain: String,
    pub key: String,
    pub noverify: bool,
    pub pathcount: Option<u64>
}

impl Request for GetPaths {
    fn op(&self) -> &'static str { "get_paths" }

    fn response_from_bytes(&self, bytes: &[u8]) -> MogResult<Response> {
        GetPathsResponse::from_bytes(bytes).map(|r| r.to_response())
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.get_paths(self).map(|r| r.to_response())
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

impl ToArgs for GetPaths {
    fn to_args(&self) -> Vec<(String, String)> {
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

/// The response to a `get_paths` request.
///
/// Looks like this:
///
/// ```text
/// request = "get_paths domain=test_domain_2&key=test/key/1&noverify=1&zone=\r\n"
/// response = "OK paths=1&path1=http://127.0.0.1:7500/dev1/0/000/001/0000001927.fid\r\n"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetPathsResponse(pub Vec<Url>);

// impl Response for GetPathsResponse {}

impl ToResponse for GetPathsResponse {
    fn to_response(self) -> Response {
        Response::GetPaths(self)
    }
}

impl FromBytes for GetPathsResponse {
    fn from_bytes(bytes: &[u8]) -> MogResult<GetPathsResponse> {
        let mut args = ArgsHash::from_bytes(bytes);
        let paths = try!(args.extract_required_int("paths", MogError::Other("No path count".to_string(), None)));
        let mut response = GetPathsResponse(Vec::new());

        for i in (1..(paths + 1)) {
            response.0.push(try!(args.extract_required_url(&format!("path{}", i), MogError::NoPath)));
        }

        Ok(response)
    }
}

impl ToArgs for GetPathsResponse {
    fn to_args(&self) -> Vec<(String, String)> {
        let mut args = vec!{
            ("paths".to_string(), self.0.len().to_string()),
        };

        for (i, url) in self.0.iter().enumerate() {
            args.push((format!("path{}", i + 1), url.to_string()));
        }

        args
    }
}

/// A `file_info` request.
///
/// Looks like this:
///
/// ```text
/// request = "file_info domain=test_domain_2&key=test/key/1\r\n"
/// response = "OK fid=1927&devcount=1&length=4&domain=test_domain_2&class=default&key=test/key/1\r\n"
/// ```
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub domain: String,
    pub key: String,
}

impl Request for FileInfo {
    fn op(&self) -> &'static str { "file_info" }

    fn response_from_bytes(&self, bytes: &[u8]) -> MogResult<Response> {
        FileInfoResponse::from_bytes(bytes).map(|r| r.to_response())
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.file_info(self).map(|r| r.to_response())
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

impl ToArgs for FileInfo {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
        }
    }
}

/// The response to a `file_info` request.
///
/// Looks like this:
///
/// ```text
/// request = "file_info domain=test_domain_2&key=test/key/1\r\n"
/// response = "OK fid=1927&devcount=1&length=4&domain=test_domain_2&class=default&key=test/key/1\r\n"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfoResponse {
    pub fid: u64,
    pub devcount: u64,
    pub length: u64,
    pub domain: String,
    pub class: String,
    pub key: String,
}

// impl Response for FileInfoResponse {}

impl ToResponse for FileInfoResponse {
    fn to_response(self) -> Response {
        Response::FileInfo(self)
    }
}

impl FromBytes for FileInfoResponse {
    fn from_bytes(bytes: &[u8]) -> MogResult<FileInfoResponse> {
        let mut args = ArgsHash::from_bytes(bytes);

        Ok(FileInfoResponse {
            fid: try!(args.extract_required_int("fid", MogError::NoFid)),
            devcount: try!(args.extract_required_int("devcount", MogError::Other("No device count".to_string(), None))),
            length: try!(args.extract_required_int("length", MogError::Other("No file size".to_string(), None))),
            domain: try!(args.extract_required_string("domain", MogError::NoDomain)),
            class: try!(args.extract_required_string("class", MogError::NoClass)),
            key: try!(args.extract_required_string("key", MogError::NoKey)),
        })
    }
}

impl ToArgs for FileInfoResponse {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("class".to_string(), self.class.clone()),
            ("fid".to_string(), self.fid.to_string()),
            ("devcount".to_string(), self.devcount.to_string()),
            ("length".to_string(), self.length.to_string()),
        }
    }
}

/// A `rename` request.
///
/// Looks like this:
///
/// ```text
/// request = "rename domain=test_domain_2&from_key=test/key/1&to_key=test/key/2\r\n"
/// response = "OK \r\n"
/// ```
#[derive(Debug, Clone)]
pub struct Rename {
    pub domain: String,
    pub from_key: String,
    pub to_key: String,
}

impl Request for Rename {
    fn op(&self) -> &'static str { "rename" }

    fn response_from_bytes(&self, _bytes: &[u8]) -> MogResult<Response> {
        Ok(Response::Empty)
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.rename(self).map(|r| r.to_response())
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

impl ToArgs for Rename {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("from_key".to_string(), self.from_key.clone()),
            ("to_key".to_string(), self.to_key.clone()),
        }
    }
}

/// An `updateclass` request.
///
/// Looks like this:
///
/// ```text
/// request = "updateclass domain=test_domain_2&key=test/key/2&class=new_class\r\n"
/// response = "OK \r\n"
/// ```
#[derive(Debug, Clone)]
pub struct UpdateClass {
    pub domain: String,
    pub key: String,
    pub new_class: String,
}

impl Request for UpdateClass {
    fn op(&self) -> &'static str { "updateclass" }

    fn response_from_bytes(&self, _bytes: &[u8]) -> MogResult<Response> {
        Ok(Response::Empty)
    }

    fn perform(&self, _backend: &Backend) -> MogResult<Response> {
        // backend.update_class(self).map(|r| r.to_response())
        Ok(Response::Empty)
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

impl ToArgs for UpdateClass {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
            ("class".to_string(), self.new_class.clone()),
        }
    }
}

/// A `delete` request.
///
/// Looks like this:
///
/// ```text
/// request = "delete domain=test_domain_2&key=test/key/2\r\n"
/// response = "OK \r\n"
/// ```
#[derive(Debug, Clone)]
pub struct Delete {
    pub domain: String,
    pub key: String,
}

impl Request for Delete {
    fn op(&self) -> &'static str { "delete" }

    fn response_from_bytes(&self, _bytes: &[u8]) -> MogResult<Response> {
        Ok(Response::Empty)
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.delete(self).map(|r| r.to_response())
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

impl ToArgs for Delete {
    fn to_args(&self) -> Vec<(String, String)> {
        vec!{
            ("domain".to_string(), self.domain.clone()),
            ("key".to_string(), self.key.clone()),
        }
    }
}

/// A `list_keys` request.
///
/// Looks like this:
///
/// ```text
/// request = "list_keys domain=development_public&prefix=Photo&after=&limit=10\r\n"
/// response = "OK key_4=Photo/120418/image/thumb&key_6=Photo/12285/image/thumb&key_5=Photo/12285/image&key_count=10&key_10=Photo/126010/image/thumb&key_7=Photo/126009/image&key_8=Photo/126009/image/thumb&key_1=Photo/1105/image&key_3=Photo/120418/image&next_after=Photo/126010/image/thumb&key_2=Photo/1105/image/thumb&key_9=Photo/126010/image\r\n"
/// ```
#[derive(Debug, Clone)]
pub struct ListKeys {
    pub domain: String,
    pub prefix: Option<String>,
    pub after: Option<String>,
    pub limit: Option<u64>,
}

impl Request for ListKeys {
    fn op(&self) -> &'static str { "list_keys" }

    fn response_from_bytes(&self, bytes: &[u8]) -> MogResult<Response> {
        ListKeysResponse::from_bytes(bytes).map(|r| r.to_response())
    }

    fn perform(&self, backend: &Backend) -> MogResult<Response> {
        backend.list_keys(self).map(|r| r.to_response())
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

impl ToArgs for ListKeys {
    fn to_args(&self) -> Vec<(String, String)> {
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

/// The response to a `list_keys` request.
///
/// Looks like this:
///
/// ```text
/// request = "list_keys domain=rn_development_public&prefix=Photo&after=&limit=10\r\n"
/// response = "OK key_4=Photo/120418/image/thumb&key_6=Photo/12285/image/thumb&key_5=Photo/12285/image&key_count=10&key_10=Photo/126010/image/thumb&key_7=Photo/126009/image&key_8=Photo/126009/image/thumb&key_1=Photo/1105/image&key_3=Photo/120418/image&next_after=Photo/126010/image/thumb&key_2=Photo/1105/image/thumb&key_9=Photo/126010/image\r\n"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListKeysResponse(pub Vec<String>);

// impl Response for ListKeysResponse {}

impl ToResponse for ListKeysResponse {
    fn to_response(self) -> Response {
        Response::ListKeys(self)
    }
}

impl FromBytes for ListKeysResponse {
    fn from_bytes(bytes: &[u8]) -> MogResult<ListKeysResponse> {
        let mut args = ArgsHash::from_bytes(bytes);
        let key_count = try!(args.extract_required_int("key_count", MogError::Other("No key count".to_string(), None)));
        let mut response = ListKeysResponse(Vec::new());

        for i in (1..(key_count + 1)) {
            response.0.push(try!(args.extract_required_string(&format!("key_{}", i), MogError::NoKey)));
        }

        Ok(response)
    }
}

impl ToArgs for ListKeysResponse {
    fn to_args(&self) -> Vec<(String, String)> {
        let mut args = vec!{
            ("key_count".to_string(), self.0.len().to_string()),
        };

        for (i, key) in self.0.iter().enumerate() {
            args.push((format!("key_{}", i+1), key.to_string()));
            if i == self.0.len() - 1 {
                args.push(("next_after".to_string(), key.to_string()));
            }
        }

        args
    }
}

/// A `noop` request.
///
/// Looks like this:
///
/// ```text
/// request = "noop \r\n"
/// response = "OK \r\n"
/// ```
#[derive(Debug)]
pub struct Noop;

impl Request for Noop {
    fn op(&self) -> &'static str { "noop" }

    fn response_from_bytes(&self, _bytes: &[u8]) -> MogResult<Response> {
        Ok(Response::Empty)
    }

    fn perform(&self, _backend: &Backend) -> MogResult<Response> {
        Ok(Response::Empty)
    }
}

impl FromBytes for Noop {
    fn from_bytes(_bytes: &[u8]) -> MogResult<Noop> {
        Ok(Noop)
    }
}

impl ToArgs for Noop {
    fn to_args(&self) -> Vec<(String, String)> {
        vec![]
    }
}
