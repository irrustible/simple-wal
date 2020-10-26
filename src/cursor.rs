use std::convert::TryFrom;
use std::fs::File;
use std::io::{Error, IoSlice, Seek, SeekFrom, Write};
use std::borrow::{Borrow, BorrowMut};
use crate::write_vectored;

pub struct Cursor {
    file: File,
    byte_offset: usize,
}

impl Cursor {
    pub fn new(file: File, byte_offset: usize) -> Cursor {
        Cursor { file, byte_offset }
    }
    pub fn into_inner(self) -> (File, usize) {
        (self.file, self.byte_offset)
    }
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }
    pub fn write_blocks(&mut self, data: &[IoSlice]) -> Result<BlockWrite, Error> {
        let start_position = self.byte_offset;
        let bytes = self.write_vectored(data)?;
        let mut blocks = 0;
        let mut b = bytes;
        for d in data.iter() {
            let len = d.len();
            if b < len {
                return Ok(BlockWrite { start_position, bytes, blocks, partial: b });
            } else {
                blocks += 1;
                b -= len;
            }
        }
        Ok(BlockWrite { start_position, bytes, blocks, partial: 0 })
    }
}

impl Borrow<File> for Cursor {
    fn borrow(&self) -> &File {
        &self.file
    }
}

impl BorrowMut<File> for Cursor {
    fn borrow_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

impl Seek for Cursor {
    fn seek(&mut self, to: SeekFrom) -> Result<u64, Error> {
        let byte_offset = self.file.seek(to)?;
        self.byte_offset = byte_offset as usize;
        Ok(byte_offset)
    }
}

impl Write for Cursor {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let wrote = self.file.write(buf)?;
        self.byte_offset += wrote;
        Ok(wrote)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn write_vectored(&mut self, data: &[IoSlice<'_>]) -> Result<usize, Error> {
        let wrote = write_vectored(&mut self.file, data)?;
        self.byte_offset += wrote;
        Ok(wrote)
    }
}

impl TryFrom<File> for Cursor {
    type Error = Error;
    fn try_from(mut file: File) -> Result<Cursor, Error> {
        let byte_offset = file.seek(SeekFrom::Current(0))? as usize;
        Ok(Cursor { file, byte_offset })
    }
}

pub struct BlockWrite {
    pub start_position: usize,
    pub blocks: usize,
    pub bytes: usize,
    pub partial: usize,
}

pub struct WAL {
    cursor: Cursor,
    partial: usize,
}

impl WAL {
    pub fn new(cursor: Cursor) -> WAL {
        WAL { cursor, partial: 0 }
    }

    pub fn write(&mut self, blocks: &[IoSlice]) -> Result<Option<BlockWrite>, Error> {
        let mut write = self.cursor.write_blocks(blocks)?;
        if write.blocks == 0 {
            self.partial += write.partial;
            Ok(None)
        } else {
            write.start_position -= self.partial;
            // so the user doesn't accidentally a silly
            write.bytes += self.partial;
            self.partial = write.partial;
            Ok(Some(write))
        }
    }
    pub fn into_inner(self) -> (Cursor, usize) {
        (self.cursor, self.partial)
    }
}
