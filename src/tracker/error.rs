use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use url::percent_encoding;

/// A result type with the error type hard-coded to `tracker::Error`.
pub type TrackerResult<'a, T> = Result<T, TrackerError>;

/// They types of error that might result from a tracker request.
#[derive(Debug)]
pub enum TrackerErrorKind {
    UnknownCommand,
    IoError(io::Error),
    Other(String),
}

impl PartialEq for TrackerErrorKind {
    fn eq(&self, other: &TrackerErrorKind) -> bool {
        use self::TrackerErrorKind::*;

        match (self, other) {
            (&UnknownCommand, &UnknownCommand) => true,
            (&IoError(_), &IoError(_)) => true,
            (&Other(_), &Other(_)) => true,
            _ => false,
        }
    }
}

impl Display for TrackerErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::TrackerErrorKind::*;

        let s = match *self {
            UnknownCommand => "unknown_command",
            IoError(_) => "io_error",
            Other(ref s) => s.as_ref(),
        };
        write!(f, "{}", s)
    }
}

/// An error coming from handling a tracker request.
#[derive(Debug)]
pub struct TrackerError {
    pub kind: TrackerErrorKind,
    description: String,
    // cause: Option<Box<error::Error>>,
}

impl TrackerError {
    pub fn error_line(&self) -> String {
        let encoded_description = percent_encoding::percent_encode(
            self.description.as_bytes(),
            percent_encoding::FORM_URLENCODED_ENCODE_SET);
        format!("ERR {} {}", self.kind, encoded_description)
    }

    pub fn unknown_command(desc: &str) -> TrackerError {
        TrackerError {
            kind: TrackerErrorKind::UnknownCommand,
            description: desc.to_string(),
            // cause: None,
        }
    }

    pub fn other(kind: &str, desc: &str) -> TrackerError {
        TrackerError {
            kind: TrackerErrorKind::Other(kind.to_string()),
            description: desc.to_string(),
            // cause: None,
        }
    }
}

impl Display for TrackerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ERR {} {}", self.kind, self.description)
    }
}

impl Error for TrackerError {
    fn description(&self) -> &str {
        &self.description
    }
}

impl From<io::Error> for TrackerError {
    fn from(io_err: io::Error) -> TrackerError {
        TrackerError {
            description: io_err.description().to_string(),
            kind: TrackerErrorKind::IoError(io_err),
            // cause: Box::new(io_err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn error_kinds() {
        assert_eq!("unknown_command", format!("{}", TrackerErrorKind::UnknownCommand));
        assert_eq!("arbitrary_error", format!("{}", TrackerErrorKind::Other("arbitrary_error".to_string())));
    }

    #[test]
    fn error_line() {
        let e = TrackerError::unknown_command("unknown command: blah");
        assert_eq!("ERR unknown_command unknown%20command%3A%20blah", e.error_line());
    }
}
