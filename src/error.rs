use iron::IronError;
use iron::status::Status;
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
    Utf8(Utf8Error),

    NoDomain,
    UnregDomain(String),
    DomainExists(String),

    NoKey,
    UnknownKey(String),
    KeyExists(String),

    UnknownCommand(Option<String>),

    NoContent(String),
}

impl MogError {
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

impl From<MogError> for IronError {
    fn from(err: MogError) -> IronError {
        use self::MogError::*;

        let modifier = match &err {
            &UnknownKey(ref k) => {
                (Status::NotFound, format!("Unknown key: {:?}\n", k))
            },
            &NoContent(ref k) => {
                (Status::NotFound, format!("No content key: {:?}\n", k))
            },
            e @ _ => {
                (Status::InternalServerError, format!("{}\n", e.description()))
            }
        };

        IronError::new(err, modifier)
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
            Io(ref io_err) => io_err.description(),
            Utf8(ref utf8_err) => utf8_err.description(),
            PoisonedMutex => "Poisoned mutex",

            NoDomain => "No domain provided",
            UnregDomain(..) => "Domain name invalid / not found",
            DomainExists(..) => "Domain already exists",

            NoKey => "No key provided",
            UnknownKey(..) => "Unknown key",
            KeyExists(..) => "Key already exists",

            UnknownCommand(..) => "Unknown command",
            NoContent(..) => "No content",
        }
    }
}
