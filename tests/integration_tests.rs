use assert_fs::{prelude::*, TempDir};
use xdf::{read_to_raw_chunks, ReadChunkError};
use std::fs;

// #[test]
// fn invalid_path() {
//     let res = read_file_to_raw_chunks("./does/not/exist.xdf");
//     assert!(matches!(res.unwrap_err(), ReadChunkError::IOError(_)))
// }

#[test]
fn empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let empty_file = temp_dir.child("empty.xdf");
    empty_file.touch().unwrap();

    //reader from path:
    let reader = fs::File::open(empty_file.path()).unwrap();

    let res = read_to_raw_chunks(reader);
    assert!(matches!(res.unwrap_err(), ReadChunkError::EOFError));
}

#[test]
fn no_magic_number() {
    let temp_dir = TempDir::new().unwrap();
    let no_magic_file = temp_dir.child("no_magic.xdf");
    no_magic_file.touch().unwrap();
    no_magic_file.write_str("NOT: a magic number").unwrap();

    //reader from path:
    let reader = fs::File::open(no_magic_file.path()).unwrap();

    let res = read_to_raw_chunks(reader);
    assert!(matches!(res.unwrap_err(), ReadChunkError::NoMagicNumberError));
}

#[test]
fn invalid_tag() {
    let bytes:Vec<u8> = vec![1, 3, 0, 0, 10];
    let res = read_to_raw_chunks(bytes.as_slice());
}
