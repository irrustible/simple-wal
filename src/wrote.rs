use crate::*;
use std::borrow::Borrow;
use std::iter::{ExactSizeIterator, Iterator};
use std::vec::Drain;

/// Information about the write. Includes:
///  * Write stats
///  * Queue stats before and after this write
///  * Information about each block written (via `Iterator`)
pub struct Wrote<'a, T>
where T: Borrow<[u8]> {
    pub before: Stats,
    pub after: Stats,
    pub wrote: Stats,
    pub(crate) iter: Drain<'a, T>,
    pub(crate) offset: &'a mut usize,
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
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Wrote<'a, T>
where T: Borrow<[u8]> {}

impl<'a, T> Drop for Wrote<'a, T> 
where T: Borrow<[u8]> {
    fn drop(&mut self) {
        while self.next().is_some() {}
    }
}
