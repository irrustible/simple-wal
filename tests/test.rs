// #![feature(test)]
use simple_wal::*;
use std::env::temp_dir;
use std::fs::remove_file;
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
    let _del = Delete(path);
    let hello: Vec<u8> = HELLO.into();
    let mut wal = WAL::create(path.as_ref(), 1024).unwrap();
    {
        wal.enqueue(hello).unwrap();
    }
    // let mut w = wal.write().unwrap();
}

#[test]
#[allow(unused_must_use)]
fn test_multi() {
    let mut path = temp_dir();
    path.push("test_simple.wal");
    let path: &Path = path.as_ref();
    remove_file(path); // in case we panic
    let _del = Delete(path);
    let hello: Vec<u8> = HELLO.into();
    let world: Vec<u8> = WORLD.into();
    let mut wal = WAL::create(path.as_ref(), 1024).unwrap();
    {
        wal.enqueue(hello).unwrap();
    }
    {
        wal.enqueue(world).unwrap();
    }
    // let mut w = wal.write().unwrap();
}
