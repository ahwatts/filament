use std::io;

pub type MogClientResult<T> = Result<T, MogClientError>;

#[derive(Debug)]
pub enum MogClientError {
    IoError(io::Error),
    NoConnection,
    NoTrackers,
}

impl From<io::Error> for MogClientError {
    fn from(ioe: io::Error) -> MogClientError {
        MogClientError::IoError(ioe)
    }
}
