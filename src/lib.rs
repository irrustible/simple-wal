#[cfg(target_family="unix")]
mod unix;
#[cfg(target_family="unix")]
pub use unix::*;

#[cfg(not(target_family="unix"))]
mod not_unix;
#[cfg(not(target_family="unix"))]
pub use not_unix::*;

mod cursor;
pub use cursor::*;

use std::io::Error;

/// Something went wrong opening a WAL
#[derive(Debug)]
pub enum OpenError {
    /// The file was bigger than the max_length provided.
    TooBig,
    /// We couldn't seek to the end of the file.
    Seek(Error),
    /// We couldn't open the file.
    Open(Error),
}

/// Something went wrong opening a WAL
#[derive(Debug)]
pub enum FromFileError {
    /// The file was bigger than the max_length provided.
    TooBig,
    /// We couldn't seek to the end of the file.
    Seek(Error),
}

impl From<FromFileError> for OpenError {
    fn from(this: FromFileError) -> OpenError {
        match this {
            FromFileError::TooBig => OpenError::TooBig,
            FromFileError::Seek(e) => OpenError::Seek(e),
        }
    }
}
/// There was an error queueing some data.
#[derive(Debug)]
pub enum EnqueueError {
    /// Enqueueing this would take us over the max_length provided.
    EndOfFile,
}
