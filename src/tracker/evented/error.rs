use mio::{Token, NotifyError};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use super::notification::Notification;

pub type EventedResult<T> = Result<T, EventedError>;

#[derive(Debug)]
pub enum EventedError {
    IoError(io::Error),
    FullNotifyQueue,
    NoListenAddr,
    StreamNotReady,
    UnknownConnection(Token),
    TooManyConnections,
    Closed,
}

impl Error for EventedError {
    fn description(&self) -> &str {
        use self::EventedError::*;

        match *self {
            IoError(ref io_err) => io_err.description(),
            FullNotifyQueue => "Notification queue is full",
            NoListenAddr => "Unable to determine address on which to listen",
            StreamNotReady => "Stream is not ready",
            UnknownConnection(_) => "Unknown connection",
            TooManyConnections => "Too many connections",
            Closed => "Notification channel has been closed",
        }
    }
}

impl Display for EventedError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::EventedError::*;

        match *self {
            IoError(ref io_err) => write!(f, "{}", io_err),
            UnknownConnection(ref token) => write!(f, "Unknown connection: {:?}", token),
            _ => f.write_str(self.description()),
        }
    }
}

impl From<io::Error> for EventedError {
    fn from(io_err: io::Error) -> EventedError {
        EventedError::IoError(io_err)
    }
}

impl From<NotifyError<Notification>> for EventedError {
    fn from(not_err: NotifyError<Notification>) -> EventedError {
        match not_err {
            NotifyError::Io(io_err) => EventedError::IoError(io_err),
            NotifyError::Full(_) => EventedError::FullNotifyQueue,
            NotifyError::Closed(_) => EventedError::Closed,
        }
    }
}
