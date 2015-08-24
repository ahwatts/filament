use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::str;
use super::error::{MogError, MogResult};
use url::form_urlencoded;

/// The different commands that the tracker implements.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    CreateDomain,

    CreateOpen,
    CreateClose,
    GetPaths,
    FileInfo,
    Rename,
    UpdateClass,
    Delete,
    ListKeys,

    Noop,
}

impl Command {
    pub fn from_optional_bytes(bytes: Option<&[u8]>) -> MogResult<Command> {
        use self::Command::*;

        match bytes.map(|bs| str::from_utf8(bs)) {
            Some(Ok(string)) if string == "create_domain" => Ok(CreateDomain),

            Some(Ok(string)) if string == "create_open" => Ok(CreateOpen),
            Some(Ok(string)) if string == "create_close" => Ok(CreateClose),
            Some(Ok(string)) if string == "get_paths" => Ok(GetPaths),
            Some(Ok(string)) if string == "file_info" => Ok(FileInfo),
            Some(Ok(string)) if string == "rename" => Ok(Rename),
            Some(Ok(string)) if string == "updateclass" => Ok(UpdateClass),
            Some(Ok(string)) if string == "delete" => Ok(Delete),
            Some(Ok(string)) if string == "list_keys" => Ok(ListKeys),

            Some(Ok(string)) if string == "noop" => Ok(Noop),

            Some(Ok(string)) if string == "" => Err(MogError::UnknownCommand(None)),
            Some(Ok(string)) => Err(MogError::UnknownCommand(Some(string.to_string()))),
            Some(Err(utf8e)) => Err(MogError::Utf8(utf8e)),
            None => Err(MogError::UnknownCommand(None)),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::Command::*;

        let op_str = match *self {
            CreateDomain => "create_domain",

            CreateOpen => "create_open",
            CreateClose => "create_close",
            GetPaths => "get_paths",
            FileInfo => "file_info",
            Rename => "rename",
            UpdateClass => "updateclass",
            Delete => "delete",
            ListKeys => "list_keys",

            Noop => "noop",
        };

        write!(f, "{}", op_str)
    }
}

/// A request to the MogileFS tracker.
#[derive(Debug)]
pub struct Request {
    pub op: Command,
    pub args: Vec<(String, String)>,
}

impl Request {
    pub fn from_bytes(bytes: &[u8]) -> MogResult<Request> {
        let mut toks = bytes.split(|&c| c == b' ');
        let command = try!(Command::from_optional_bytes(toks.next()));

        Ok(Request {
            op: command,
            args: form_urlencoded::parse(toks.next().unwrap_or(b"")),
        })
    }

    pub fn args_hash<'a>(&'a self) -> HashMap<&'a str, &'a str> {
        self.args.iter().fold(HashMap::new(), |mut m, &(ref k, ref v)| {
            *m.entry(k).or_insert(v) = v; m
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::MogError;
    use super::*;

    #[test]
    fn command_from_no_bytes() {
        assert!(matches!(Command::from_optional_bytes(None),
                         Err(MogError::UnknownCommand(None))));

        assert!(matches!(Command::from_optional_bytes(Some(b"")),
                         Err(MogError::UnknownCommand(None))));
    }

    #[test]
    fn unknown_command() {
        assert!(matches!(Command::from_optional_bytes(Some(b"this_command_doesnt_exist")),
                         Err(MogError::UnknownCommand(Some(ref s))) if s == "this_command_doesnt_exist"));
    }

    #[test]
    fn known_command() {
        assert!(matches!(Command::from_optional_bytes(Some(b"create_open")),
                         Ok(Command::CreateOpen)));
    }

    #[test]
    fn request_from_no_bytes() {
        assert!(matches!(Request::from_bytes(b""),
                Err(MogError::UnknownCommand(None))));
    }

    #[test]
    fn request_with_unknown_command() {
        assert!(matches!(Request::from_bytes("this_command_doesnt_exist key1=val1&domain=foo".as_bytes()),
                         Err(MogError::UnknownCommand(Some(ref s))) if s == "this_command_doesnt_exist"));
    }

    #[test]
    fn request_with_no_args() {
        let request = Request::from_bytes(b"create_open");
        assert!(request.is_ok());
        let request = request.unwrap();
        assert_eq!(Command::CreateOpen, request.op);
        assert!(request.args.is_empty());
    }

    #[test]
    fn request_with_args() {
        let request = Request::from_bytes(b"create_open domain=foo&key=test/key/1");
        assert!(request.is_ok());
        let request = request.unwrap();
        assert_eq!(Command::CreateOpen, request.op);
        assert_eq!(2, request.args.len());
        assert!(request.args.iter().find(|&&(ref k, ref v)| k == "domain" && v == "foo").is_some());
        assert!(request.args.iter().find(|&&(ref k, ref v)| k == "key" && v == "test/key/1").is_some());
    }
}
