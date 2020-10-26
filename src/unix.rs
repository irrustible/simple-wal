use std::fs::{File, OpenOptions};
use std::io::{Error, IoSlice, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::convert::TryInto;
use std::path::Path;
use libc;
use crate::*;

// linux and netbsd are quite good as posix goes. apple's website's
// ancient manpages says it's shit, but apparently it isn't.
#[cfg(any(target_os="linux",target_os="netbsd",target_os="macos"))]
const OPEN_FLAGS: i32 = libc::O_DSYNC;
#[cfg(any(target_os="freebsd",target_os="openbsd"))]
const OPEN_FLAGS: i32 = libc::O_SYNC; // freebsd and openbsd score ok
#[cfg(target_os="dragonfly")]
const OPEN_FLAGS: i32 = libc::O_FSYNC; // dragonfly: 3/10, see me after class

/// Something went wrong during creating a WAL
#[derive(Debug)]
pub enum CreateError {
    /// An error occurred during opening the file.
    Open(Error),
    /// We couldn't find the parent directory of the path. Did you pass us '/'?
    NoParent,
    /// We couldn't open the parent directory (permissions?)
    OpenParent(Error),
    /// There was an error syncing the parent directory.
    Sync(Error),
}

fn open_parent_dir(path: &Path) -> Result<File, CreateError> {
    let dirname = path.parent().ok_or(CreateError::NoParent)?;
    File::open(dirname).map_err(|e| CreateError::OpenParent(e))
}

pub fn create_new_wal_file(path: &Path) -> Result<Cursor, CreateError> {
    let parent = open_parent_dir(path)?;
    let file = OpenOptions::new()
        .create_new(true).write(true).custom_flags(OPEN_FLAGS)
        .open(path).map_err(|e| CreateError::Open(e))?;
    // Because POSIX is a delight, we have to fsync the directory
    // handle to be sure the file exists in the directory...
    parent.sync_all().map_err(|e| CreateError::Sync(e))?;
    Ok(Cursor::new(file, 0))
}

pub fn open_existing_wal_file(path: &Path) -> Result<Cursor, OpenError> {
    let file = OpenOptions::new()
        .read(true).append(true).custom_flags(OPEN_FLAGS)
        .open(path).map_err(|e| OpenError::Open(e))?;
    file.try_into().map_err(OpenError::Seek)
}

pub(crate) fn write_vectored(file: &mut File, data: &[IoSlice]) -> Result<usize, Error> {
    file.write_vectored(data)
}
