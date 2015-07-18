use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::sync::{MutexGuard, PoisonError};
use super::super::common::Backend;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    PoisonedMutex,
    UnknownKey,
    NoContent,
}

impl<'a> From<PoisonError<MutexGuard<'a, Backend>>> for StorageError {
    fn from (_: PoisonError<MutexGuard<'a, Backend>>) -> StorageError {
        StorageError::PoisonedMutex
    }
}

impl From<io::Error> for StorageError {
    fn from(io_err: io::Error) -> StorageError {
        StorageError::Io(io_err)
    }
}

impl PartialEq for StorageError {
    fn eq(&self, other: &StorageError) -> bool {
        use self::StorageError::*;

        match (self, other) {
            (&Io(_), &Io(_)) => true,
            (&PoisonedMutex, &PoisonedMutex) => true,
            (&UnknownKey, &UnknownKey) => true,
            (&NoContent, &NoContent) => true,
            _ => false,
        }
    }
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::StorageError::*;
        match *self {
            Io(ref io_err) => write!(f, "{}", io_err),
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl Error for StorageError {
    fn description(&self) -> &str {
        use self::StorageError::*;
        match *self {
            Io(ref io_err) => io_err.description(),
            PoisonedMutex => "Poisoned mutex",
            UnknownKey => "Unknown key",
            NoContent => "No content",
        }
    }
}
