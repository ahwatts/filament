//! Common error and result types for mogilefsd.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::str::{self, Utf8Error};
use std::sync::mpsc::{SendError, RecvError};
use std::sync::{MutexGuard, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use super::request::Renderable;
use super::util::ToUrlencodedString;
use url::percent_encoding::{self, FORM_URLENCODED_ENCODE_SET};

/// A specialization of `Result` with the error type hard-coded to
/// `MogError`.
pub type MogResult<T> = Result<T, MogError>;

/// The error types that mogilefsd can produce.
#[derive(Debug)]
pub enum MogError {
    Database(String),
    DomainExists(String),
    Io(io::Error),
    KeyExists(String),
    NoClass,
    NoConnection,
    NoContent(String),
    NoDevid,
    NoDomain,
    NoFid,
    NoKey,
    NoPath,
    NoTrackers,
    InvalidMindevcount,
    Other(String, Option<String>),
    PoisonedMutex,
    RecvError,
    SendError,
    UnknownCommand(Option<String>),
    UnknownKey(String),
    UnregDomain(String),
    UnregClass(String),
    UnknownCode(String),
    Utf8(Utf8Error),
    BadResponse,
    StorageError(Option<String>),
}

impl MogError {
    /// Return the string used in the MogileFS tracker response for
    /// the error.
    pub fn error_kind(&self) -> &str {
        use self::MogError::*;

        match *self {
            DomainExists(..) => "domain_exists",
            InvalidMindevcount => "invalid_mindevcount",
            KeyExists(..) => "key_exists",
            NoDomain => "no_domain",
            NoKey => "no_key",
            UnknownCommand(..) => "unknown_command",
            UnknownKey(..) => "unknown_key",
            UnregClass(..) => "unreg_class",
            UnregDomain(..) => "unreg_domain",

            Other(ref op, _) => op,

            Database(..) => "db_error",
            Io(..) => "io_error",
            NoClass => "no_class",
            NoConnection => "no_connection",
            NoContent(..) => "no_content",
            NoDevid => "no_devid",
            NoFid => "no_fid",
            NoPath => "no_path",
            NoTrackers => "no_trackers",
            PoisonedMutex => "poisoned_mutex",
            SendError => "send_error",
            RecvError => "recv_error",
            UnknownCode(..) => "unknown_code",
            Utf8(..) => "utf8_error",
            BadResponse => "bad_response",
            StorageError(..) => "storage_error",
        }
    }

    /// Constructs a `MogError` from the bytes provided.
    pub fn from_bytes(bytes: &[u8]) -> MogError {
        use self::MogError::*;

        let mut toks = bytes.split(|&b| b == b' ');
        let op = toks.next();
        let msg = toks.next().map(|m| {
            percent_encoding::lossy_utf8_percent_decode(m)
                .replace("+", " ")
        });

        match op.map(|o| str::from_utf8(o)) {
            Some(Ok("invalid_mindevcount")) => InvalidMindevcount,
            Some(Ok("no_class")) => NoClass,
            Some(Ok("no_devid")) => NoDevid,
            Some(Ok("no_domain")) => NoDomain,
            Some(Ok("no_fid")) => NoFid,
            Some(Ok("no_path")) => NoPath,
            Some(Ok("unknown_command")) => UnknownCommand(msg),
            Some(Ok("unknown_key")) => UnknownKey(msg.unwrap_or(String::new())),
            Some(Ok("unreg_domain")) => UnregDomain(msg.unwrap_or(String::new())),
            Some(Ok("unreg_class")) => UnregClass(msg.unwrap_or(String::new())),
            Some(Ok(s)) => Other(s.to_string(), msg),
            Some(Err(utf8e)) => Utf8(utf8e),
            None => UnknownCommand(None),
        }
    }
}

impl<'a, T> From<PoisonError<RwLockReadGuard<'a, T>>> for MogError {
    fn from (_: PoisonError<RwLockReadGuard<'a, T>>) -> MogError {
        MogError::PoisonedMutex
    }
}

impl<'a, T> From<PoisonError<RwLockWriteGuard<'a, T>>> for MogError {
    fn from (_: PoisonError<RwLockWriteGuard<'a, T>>) -> MogError {
        MogError::PoisonedMutex
    }
}

impl<'a, T> From<PoisonError<MutexGuard<'a, T>>> for MogError {
    fn from(_: PoisonError<MutexGuard<'a, T>>) -> MogError {
        MogError::PoisonedMutex
    }
}

impl<T> From<SendError<T>> for MogError {
    fn from(_: SendError<T>) -> MogError {
        MogError::SendError
    }
}

impl From<RecvError> for MogError {
    fn from(_: RecvError) -> MogError {
        MogError::RecvError
    }
}

impl From<io::Error> for MogError {
    fn from(io_err: io::Error) -> MogError {
        MogError::Io(io_err)
    }
}

impl From<Utf8Error> for MogError {
    fn from(utf8_err: Utf8Error) -> MogError {
        MogError::Utf8(utf8_err)
    }
}

impl Display for MogError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::MogError::*;
        match *self {
            Database(ref d) => write!(f, "{}", d),
            Io(ref io_err) => write!(f, "{}", io_err),
            Utf8(ref utf8_err) => write!(f, "{}", utf8_err),

            UnregDomain(ref d) => write!(f, "Domain name {:?} invalid / not found", d),
            UnregClass(ref d) => write!(f, "Class name {:?} invalid / not found", d),
            DomainExists(ref d) => write!(f, "That domain already exists: {:?}", d),

            UnknownKey(ref d) => write!(f, "Unknown key: {:?}", d),
            KeyExists(ref d) => write!(f, "Target key name {:?} already exists, can't overwrite.", d),

            UnknownCommand(ref d) => write!(f, "Unknown command: {:?}", d),
            NoContent(ref d) => write!(f, "No content for key: {:?}", d),

            Other(ref op, ref desc) => write!(f, "{} {}", op, desc.clone().unwrap_or_default()),
            UnknownCode(ref c) => write!(f, "Unknown code: {:?}", c),
            StorageError(ref os) => write!(f, "Storage error: {:?}", os),

            _ => write!(f, "{}", self.description()),
        }
    }
}

impl Error for MogError {
    fn description(&self) -> &str {
        use self::MogError::*;
        match *self {
            Database(..) => "Database error",
            DomainExists(..) => "Domain already exists",
            Io(ref io_err) => io_err.description(),
            KeyExists(..) => "Key already exists",
            InvalidMindevcount => "The mindevcount must be at least 1",
            NoClass => "No class provided",
            NoConnection => "Could not connect to tracker",
            NoContent(..) => "No content",
            NoDevid => "No device ID provided",
            NoDomain => "No domain provided",
            NoFid => "No file ID provided",
            NoKey => "No key provided",
            NoPath => "No path provided",
            NoTrackers => "No trackers provided",
            Other(..) => "Other error",
            PoisonedMutex => "Poisoned mutex",
            RecvError => "Error receiving response",
            SendError => "Error sending request",
            UnknownCode(..) => "Unknown response code",
            UnknownCommand(..) => "Unknown command",
            UnknownKey(..) => "Unknown key",
            UnregDomain(..) => "Domain name invalid / not found",
            UnregClass(..) => "Class name invalid / not found",
            BadResponse => "Wrong response type",
            StorageError(..) => "Storage error",
            Utf8(ref utf8_err) => utf8_err.description(),
        }
    }
}

impl ToUrlencodedString for MogError {
    fn to_urlencoded_string(&self) -> String {
        percent_encoding::percent_encode(self.description().as_bytes(), FORM_URLENCODED_ENCODE_SET)
    }
}

impl Renderable for MogError {
    fn render(&self) -> String {
        format!("ERR {} {}", self.error_kind(), self.to_urlencoded_string())
    }
}
