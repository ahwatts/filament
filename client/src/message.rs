use std::collections::HashMap;
use url::form_urlencoded;

#[derive(Debug)]
pub enum Request {
    FileInfo { domain: String, key: String },
}

impl Request {
    pub fn file_info(domain: &str, key: &str) -> Request {
        Request::FileInfo { domain: domain.to_string(), key: key.to_string() }
    }

    pub fn line(&self) -> String {
        format!("{} {}", self.op(), form_urlencoded::serialize(self.args()))
    }

    fn op(&self) -> &'static str {
        use self::Request::*;

        match self {
            &FileInfo {..} => "file_info",
        }
    }

    fn args(&self) -> Vec<(&str, &str)> {
        use self::Request::*;

        match self {
            &FileInfo { ref domain, ref key } => vec![ ("domain", domain), ("key", key) ],
        }
    }
}

#[derive(Debug)]
pub struct Response(Result<HashMap<String, String>, String>);

impl Response {
    pub fn from_line(line: &str) -> Response {
        println!("Response from MogileFS: {:?}", line);

        // A good response looks like this:
        // OK class=song&devcount=1&domain=rn_development_private&key=Song/225322/image&fid=53&length=2965632\r\n

        let trimmed = line.trim_right();
        let mut tokens = trimmed.split(" ");
        let status = tokens.next().unwrap_or("ERR");
        let args = tokens.next().unwrap_or("no_args");
        let msg = tokens.next().unwrap_or("No message");

        let mut rv = HashMap::new();
        for (k, v) in form_urlencoded::parse(trimmed.as_bytes()).into_iter() {
            println!("k = {:?} v = {:?}", k, v);
            rv.entry(k).or_insert(v);
            println!("    rv = {:?}", rv);
        }

        // Response(rv)
        Response(Ok(HashMap::new()))
    }

    #[inline]
    pub fn as_hash(&self) -> Option<&HashMap<String, String>> {
        match self.0 {
            Ok(ref hash) => Some(hash),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_info_request() {
        let fi = Request::file_info("domain with space", "test/key/1");
        assert_eq!("file_info", fi.op());
        assert_eq!(vec![ ("domain", "domain with space"), ("key", "test/key/1") ], fi.args());
        assert_eq!("file_info domain=domain+with+space&key=test%2Fkey%2F1", fi.line());
    }

    #[test]
    fn file_info_response() {
        let line = "OK class=song&devcount=1&domain=rn_development_private&key=Song/225322/image&fid=53&length=2965632\r\n";
        let res = Response::from_line(line);
        let res_hash = res.as_hash();

        println!("res = {:?}", res);

        // assert_eq!(res_hash["class"], "song");
        // assert_eq!(res_hash["devcount"], "1");
    }
}
