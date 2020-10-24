use std::io::Error;

mod wal;
pub use wal::WAL;

mod wrote;
pub use wrote::Wrote;

/// Something went wrong during creating a WAL
#[derive(Debug)]
pub enum CreateError {
    /// An error occurred during opening the file.
    Open(Error),
    #[cfg(unix)]
    /// We couldn't find the parent directory of the path. Did you pass us '/'?
    BadPath,
    #[cfg(unix)]
    /// We couldn't open the parent directory (permissions?)
    OpenDir(Error),
    #[cfg(unix)]
    /// There was an error syncing the parent directory.
    Sync(Error),
}

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

/// There was an error queueing some data.
#[derive(Debug)]
pub enum EnqueueError {
    /// Enqueueing this would take us over the max_length provided.
    EndOfFile,
}

/// There was an error writing some data.
#[derive(Debug)]
pub enum WriteError {
    /// The data was never written.
    Unwritten(Error),
    /// The data was written, but syncing failed.
    Unsynced(Error, Stats),
}

/// The size of a queue or a write in blocks and bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// The number of blocks.
    pub blocks: usize,
    /// The number of bytes.
    pub bytes: usize,
}

impl Stats {
    fn new(blocks: usize, bytes: usize) -> Stats {
        Stats { blocks, bytes }
    }
}

