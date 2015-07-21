use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::str::Utf8Error;
use std::sync::{MutexGuard, PoisonError};
use super::common::Backend;

pub type MogResult<T> = Result<T, MogError>;

#[derive(Debug)]
pub enum MogError {
    Io(io::Error),
    PoisonedMutex,

    DuplicateDomain(Option<String>),
    DuplicateClass(Option<String>),
    DuplicateKey(Option<String>),

    UnknownDomain(Option<String>),
    UnknownClass(Option<String>),
    UnknownKey(Option<String>),

    UnknownCommand(Option<String>),
    Utf8(Utf8Error),

    NoContent(Option<String>),
}

impl MogError {
    pub fn error_kind(&self) -> &str {
        use self::MogError::*;

        match *self {
            UnknownCommand(..) => "unknown_command",
            DuplicateDomain(..) => "domain_exists",
            _ => "other_error",
        }
    }
}

impl<'a> From<PoisonError<MutexGuard<'a, Backend>>> for MogError {
    fn from (_: PoisonError<MutexGuard<'a, Backend>>) -> MogError {
        MogError::PoisonedMutex
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
            Io(ref io_err) => write!(f, "{}", io_err),
            Utf8(ref utf8_err) => write!(f, "{}", utf8_err),
            DuplicateDomain(ref d) => write!(f, "That domain already exists: {:?}", d),
            DuplicateClass(ref d) => write!(f, "Duplicate class: {:?}", d),
            DuplicateKey(ref d) => write!(f, "Duplicate key: {:?}", d),
            UnknownDomain(ref d) => write!(f, "Unknown domain: {:?}", d),
            UnknownClass(ref d) => write!(f, "Unknown class: {:?}", d),
            UnknownKey(ref d) => write!(f, "Unknown key: {:?}", d),
            UnknownCommand(ref d) => write!(f, "Unknown command: {:?}", d),
            NoContent(ref d) => write!(f, "No content for key: {:?}", d),
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl Error for MogError {
    fn description(&self) -> &str {
        use self::MogError::*;
        match *self {
            Io(ref io_err) => io_err.description(),
            Utf8(ref utf8_err) => utf8_err.description(),
            PoisonedMutex => "Poisoned mutex",
            DuplicateDomain(..) => "Duplicate domain",
            DuplicateClass(..) => "Duplicate class",
            DuplicateKey(..) => "Duplicate key",
            UnknownDomain(..) => "Unknown domain",
            UnknownClass(..) => "Unknown class",
            UnknownKey(..) => "Unknown key",
            UnknownCommand(..) => "Unknown command",
            NoContent(..) => "No content",
        }
    }
}
