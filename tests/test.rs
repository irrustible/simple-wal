// #![feature(test)]
use simple_wal::*;
use std::env::temp_dir;
use std::fs::remove_file;
use std::io::IoSlice;
use std::path::Path;

struct Delete<'a>(&'a Path);
impl<'a> Drop for Delete<'a> {
    fn drop(&mut self) {
        remove_file(self.0).expect("to delete temporary file");
    }
}
const HELLO: &'static [u8] = b"hello";
const WORLD: &'static [u8] = b"world";

#[test]
#[allow(unused_must_use)]
fn test_simple() {
    let mut path = temp_dir();
    path.push("test_simple.wal");
    let path: &Path = path.as_ref();
    remove_file(path); // in case we panic
    let cursor = create_new_wal_file(path).unwrap();
    let mut wal = WAL::new(cursor);
    let _del = Delete(path);
    let slices: [IoSlice; 2] = [ IoSlice::new(HELLO), IoSlice::new(WORLD) ];
    let w = wal.write(&slices[..]).unwrap().unwrap();
    assert_eq!(w.blocks, 2);
    assert_eq!(w.bytes, 10);
    assert_eq!(w.start_position, 0);
    assert_eq!(w.partial, 0);
}

// #[test]
// #[allow(unused_must_use)]
// fn test_multi() {
//     let mut path = temp_dir();
//     path.push("test_multi.wal");
//     let path: &Path = path.as_ref();
//     remove_file(path); // in case we panic
//     let _del = Delete(path);
//     let mut wal = WAL::create(path.as_ref(), 1024).unwrap();
//     {
//         wal.enqueue(HELLO).unwrap();
//     }
// }
