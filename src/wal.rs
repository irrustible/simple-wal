use crate::*;
use std::borrow::Borrow;
use std::fs::{File, OpenOptions};
use std::io::{IoSlice, Seek, SeekFrom, Write};
use std::path::Path;

/// A free-form Write-Ahead Log based on an fsynced append-only log file.
///
/// The WAL is essentially a queue of byte buffers (`T: Borrow<[u8]>`).
/// You can push onto the queue with `enqueue()` and flush with `write()`.
///
/// When enqueuing, the WAL takes temporary ownership of the
/// buffer. When that buffer has been written safely to disk in its
/// entirety, it returns it for reuse.
pub struct WAL<'a, T> {
    file: File,
    offset: usize,
    queued: usize,
    max_length: usize,
    sources: Vec<T>,
    slices: Vec<IoSlice<'a>>,
}

impl<'a, T> WAL<'a, T>
where T: Borrow<[u8]> {
    #[cfg(unix)]
    /// Creates a new WAL at the provided path, configuring the maximum file size.
    /// Errors out if there was an I/O error or the file already exists.
    pub fn create(path: &Path, max_length: usize) -> Result<Self, CreateError> {
        let dirname = path.parent().ok_or(CreateError::BadPath)?;
        let dir = File::open(dirname).map_err(|e| CreateError::OpenDir(e))?;
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        dir.sync_all().map_err(|e| CreateError::Sync(e))?;
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources: Vec::new(), slices: Vec::new() })
    }

    #[cfg(not(unix))]
    /// Creates a new WAL at the provided path, configuring the maximum file size.
    /// Errors out if there was an I/O error or the file already exists.
    pub fn create(path: &Path, max_length: usize) -> Result<Self, CreateError> {
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources: Vec::new(), slices: Vec::new() })
    }

    #[cfg(unix)]
    /// `create()`, but preallocating our internal vectors to minimise reallocation.
    /// Errors out if there was an I/O error or the file already exists.
    pub fn create_with_capacity(path: &Path, max_length: usize, capacity: usize) -> Result<Self, CreateError> {
        let dirname = path.parent().ok_or(CreateError::BadPath)?;
        let dir = File::open(dirname).map_err(|e| CreateError::OpenDir(e))?;
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        dir.sync_all().map_err(|e| CreateError::Sync(e))?;
        let sources = Vec::with_capacity(capacity);
        let slices = Vec::with_capacity(capacity);
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources, slices })
    }

    #[cfg(not(unix))]
    /// `create()`, but preallocating our internal vectors to minimise reallocation.
    /// Errors out if there was an I/O error or the file already exists.
    pub fn create_with_capacity(path: &Path, max_length: usize, capacity: usize) -> Result<Self, CreateError> {
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        let sources = Vec::with_capacity(capacity);
        let slices = Vec::with_capacity(capacity);
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources, slices })
    }

    /// Opens an existing WAL at the provided path, configuring the maximum file size.
    /// Errors out if there was an I/O error or the file is already at or over its maximum size.
    pub fn open(path: &Path, max_length: usize) -> Result<Self, OpenError> {
        let mut file = OpenOptions::new().append(true).open(path).map_err(|e| OpenError::Open(e))?;
        let offset = file.seek(SeekFrom::End(0)).map_err(OpenError::Seek)? as usize;
        if offset >= max_length {
            Err(OpenError::TooBig)
        } else {
            Ok(WAL { file, offset, max_length, queued: 0, sources: Vec::new(), slices: Vec::new() })
        }
    }

    /// `open()`, but preallocating our internal vectors to minimise reallocation.
    /// Errors out if there was an I/O error or the file is already at or over its maximum size.
    pub fn open_with_capacity(path: &Path, max_length: usize, capacity: usize) -> Result<Self, OpenError> {
        let mut file = OpenOptions::new().append(true).open(path).map_err(|e| OpenError::Open(e))?;
        let offset = file.seek(SeekFrom::End(0)).map_err(OpenError::Seek)? as usize;
        if offset >= max_length {
            Err(OpenError::TooBig)
        } else {
            let sources = Vec::with_capacity(capacity);
            let slices = Vec::with_capacity(capacity);
            Ok(WAL { file, offset, max_length, queued: 0, sources, slices })
        }
    }

    /// Append an item to the queue for the next write.
    /// Fails if the data would take us over the max_length.
    pub fn enqueue(&'a mut self, data: T) -> Result<usize, EnqueueError> {
        let new_len = data.borrow().len();
        let cur_len = self.offset + self.queued;
        if (cur_len + new_len) > self.max_length {
            Err(EnqueueError::EndOfFile)
        } else {
            let i = self.sources.len();
            self.sources.push(data);
            self.slices.push(IoSlice::new(self.sources[i].borrow()));
            self.queued += new_len;
            Ok(cur_len)
        }
    }
    
    /// Writes and syncs as much of the queue as possible (in order).
    /// When no I/O error is encountered, returns information about the write.
    pub fn write(&'a mut self) -> Result<Wrote<'a, T>, WriteError> {
        let old_queue_blocks = self.slices.len();
        let old_queue_bytes = self.queued;
        let before = Stats::new(old_queue_blocks, old_queue_bytes);
        if old_queue_blocks == 0 {
            Ok(Wrote {
                before,
                after: before,
                wrote: Stats::new(0, 0),
                iter: self.sources.drain(0..0),
                offset: &mut self.offset,
            })
        } else {
            let wrote_bytes = self.file.write_vectored(&mut self.slices).map_err(WriteError::Unwritten)?;
            self.queued -= wrote_bytes;
            if wrote_bytes == 0 {
                Ok(Wrote {
                    before,
                    after: before,
                    wrote: Stats::new(0, 0),
                    iter: self.sources.drain(0..0),
                    offset: &mut self.offset,
                })
            } else {
                let new_queue_bytes = old_queue_bytes - wrote_bytes;
                let new_queue_blocks = self.slices.len();
                let wrote_blocks = old_queue_blocks - new_queue_blocks;
                match File::sync_all(&self.file).map_err(|e| WriteError::Unsynced(e, Stats::new(wrote_blocks, wrote_bytes))) {
                    Ok(()) => {
                        Ok(Wrote {
                            before,
                            after: Stats::new(new_queue_blocks, new_queue_bytes),
                            wrote: Stats::new(wrote_blocks, wrote_bytes),
                            iter: self.sources.drain(..wrote_blocks),
                            offset: &mut self.offset,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}
