use std::fs::{File, OpenOptions};
use std::io::Error;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use libc;
use libc::*;

/// Something went wrong during creating a WAL
#[derive(Debug)]
pub enum CreateError {
    /// An error occurred during opening the file.
    Open(Error),
}

pub fn create_new_wal_file(path: &Path) -> Result<Cursor, CreateError> {
    OpenOptions::new()
        .create_new(true).read(true).write(true)
        .open(path).map_err(|e| CreateError::Open(e))
}

pub fn open_existing_wal_file(path: &Path) -> Result<Cursor, OpenError> {
    let file = OpenOptions::new()
        .read(true).write(true)
        .open(path).map_err(|e| OpenError::Open(e));
    Ok(file.try_into())
}

pub(crate) fn write_vectored(file: &mut File, data: &[IoSlice]) -> Result<usize, Error> {
    let size = file.write_vectored(data)?;
    file.sync_data()?;
    Ok(size)
}
