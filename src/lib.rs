use std::borrow::Borrow;
use std::fs::{File, OpenOptions};
use std::io::{Error, IoSlice, Seek, SeekFrom, Write};
use std::iter::Iterator;
use std::path::Path;
use std::vec::Drain;

pub enum CreateError {
    Open(Error),
    #[cfg(unix)]
    /// The Path
    BadPath,
    #[cfg(unix)]
    OpenDir(Error),
    #[cfg(unix)]
    Sync(Error),
}

pub enum OpenError {
    TooBig,
    Seek(Error),
    Open(Error),
}

pub enum EnqueueError {
    EndOfFile,
}

pub enum WriteError {
    Unwritten(Error),
    Unsynced(Error, usize, usize),
}

pub struct WAL<'a, T> {
    file: File,
    offset: usize,
    queued: usize,
    max_length: usize,
    sources: Vec<T>,
    slices: Vec<IoSlice<'a>>,
}

pub struct Wrote<'a, T>
where T: Borrow<[u8]> {
    iter: Drain<'a, T>,
    offset: &'a mut usize,
}

impl<'a, T> Iterator for Wrote<'a, T>
where T: Borrow<[u8]> {
    type Item = (T, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.iter.next()?;
        let offset = *self.offset;
        *self.offset += i.borrow().len();
        Some((i, offset))
    }
}

impl<'a, T> Drop for Wrote<'a, T> 
where T: Borrow<[u8]> {
    fn drop(&mut self) {
        for _ in self {}
    }
}

impl<'a, T> WAL<'a, T>
where T: Borrow<[u8]> {
    #[cfg(unix)]
    pub fn create(path: &Path, max_length: usize) -> Result<Self, CreateError> {
        let dirname = path.parent().ok_or(CreateError::BadPath)?;
        let dir = File::open(dirname).map_err(|e| CreateError::OpenDir(e))?;
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        dir.sync_all().map_err(|e| CreateError::Sync(e))?;
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources: Vec::new(), slices: Vec::new() })
    }

    #[cfg(not(unix))]
    pub fn create(path: &Path, max_length: usize) -> Result<Self, CreateError> {
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources: Vec::new(), slices: Vec::new() })
    }

    #[cfg(unix)]
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
    pub fn create_with_capacity(path: &Path, max_length: usize, capacity: usize) -> Result<Self, CreateError> {
        let file = OpenOptions::new().create_new(true).open(path).map_err(|e| CreateError::Open(e))?;
        let sources = Vec::with_capacity(capacity);
        let slices = Vec::with_capacity(capacity);
        Ok(WAL { file, offset: 0, max_length, queued: 0, sources, slices })
    }

    pub fn open(path: &Path, max_length: usize) -> Result<Self, OpenError> {
        let mut file = OpenOptions::new().append(true).open(path).map_err(|e| OpenError::Open(e))?;
        let offset = file.seek(SeekFrom::End(0)).map_err(OpenError::Seek)? as usize;
        if offset >= max_length {
            Err(OpenError::TooBig)
        } else {
            Ok(WAL { file, offset, max_length, queued: 0, sources: Vec::new(), slices: Vec::new() })
        }
    }

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
    
    pub fn write(&'a mut self) -> Result<Wrote<'a, T>, WriteError> {
        match sync_write_vectored(&mut self.file, &mut self.slices) {
            Ok((blocks, bytes)) => {
                self.queued -= bytes;
                Ok(Wrote { iter: self.sources.drain(..blocks), offset: &mut self.offset })
            }
            Err(e) => {
                if let WriteError::Unsynced(_, _, bytes) = e {
                    self.queued -= bytes;
                }
                Err(e)
            }
        }
    }
}

pub fn sync_write_vectored<'a>(file: &mut File, batch: &mut [IoSlice<'a>]) -> Result<(usize, usize), WriteError> {
    let len = batch.len();
    if len == 0 { return Ok((0, 0)); }
    let bytes = file.write_vectored(batch).map_err(WriteError::Unwritten)?;
    let blocks = batch.len() - len;
    File::sync_all(file).map_err(|e| WriteError::Unsynced(e, blocks, bytes))?;
    Ok((blocks, bytes))
}
