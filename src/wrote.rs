use crate::*;
use std::borrow::Borrow;
use std::iter::{ExactSizeIterator, Iterator};
use std::vec::Drain;

/// Information about the write. Includes:
///  * Write stats
///  * Queue stats before and after this write
///  * Information about each block written (via `Iterator`)
#[derive(Debug)]
pub struct Wrote<'a> {
    pub before: Stats,
    pub after: Stats,
    pub wrote: Stats,
    pub(crate) iter: Drain<'a, &'a [u8]>,
    pub(crate) offset: &'a mut usize,
}

impl<'a> Iterator for Wrote<'a> {
    type Item = (&'a [u8], usize);
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

impl<'a> ExactSizeIterator for Wrote<'a> {}

impl<'a> Drop for Wrote<'a> {
    fn drop(&mut self) {
        while self.next().is_some() {}
    }
}
