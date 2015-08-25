//! Common error and result types for mogilefsd.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::str::Utf8Error;
use std::sync::{MutexGuard, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use std::sync::mpsc::{SendError, RecvError};

/// A specialization of `Result` with the error type hard-coded to
/// `MogError`.
pub type MogResult<T> = Result<T, MogError>;

/// The error types that mogilefsd can produce.
#[derive(Debug)]
pub enum MogError {
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
    PoisonedMutex,
    RecvError,
    SendError,
    UnknownCommand(Option<String>),
    UnknownKey(String),
    UnregDomain(String),
    Utf8(Utf8Error),
}

impl MogError {
    /// Return the string used in the MogileFS tracker response for
    /// the error.
    pub fn error_kind(&self) -> &str {
        use self::MogError::*;

        match *self {
            NoDomain => "no_domain",
            UnregDomain(..) => "unreg_domain",
            DomainExists(..) => "domain_exists",

            NoKey => "no_key",
            UnknownKey(..) => "unknown_key",
            KeyExists(..) => "key_exists",

            UnknownCommand(..) => "unknown_command",

            _ => "other_error",
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
            Io(ref io_err) => write!(f, "{}", io_err),
            Utf8(ref utf8_err) => write!(f, "{}", utf8_err),

            UnregDomain(ref d) => write!(f, "Domain name {:?} invalid / not found", d),
            DomainExists(ref d) => write!(f, "That domain already exists: {:?}", d),

            UnknownKey(ref d) => write!(f, "Unknown key: {:?}", d),
            KeyExists(ref d) => write!(f, "Target key name {:?} already exists, can't overwrite.", d),

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
            DomainExists(..) => "Domain already exists",
            Io(ref io_err) => io_err.description(),
            KeyExists(..) => "Key already exists",
            NoClass => "No class provided",
            NoConnection => "Could not connect to tracker",
            NoContent(..) => "No content",
            NoDevid => "No device ID provided",
            NoDomain => "No domain provided",
            NoFid => "No file ID provided",
            NoKey => "No key provided",
            NoPath => "No path provided",
            NoTrackers => "No trackers provided",
            PoisonedMutex => "Poisoned mutex",
            RecvError => "Error receiving response",
            SendError => "Error sending request",
            UnknownCommand(..) => "Unknown command",
            UnknownKey(..) => "Unknown key",
            UnregDomain(..) => "Domain name invalid / not found",
            Utf8(ref utf8_err) => utf8_err.description(),
        }
    }
}
