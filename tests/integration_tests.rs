use assert_fs::{prelude::*, TempDir};
use std::fs;
use xdf::{read_to_raw_chunks, ReadChunkError};

// #[test]
// fn invalid_path() {
//     let res = read_file_to_raw_chunks("./does/not/exist.xdf");
//     assert!(matches!(res.unwrap_err(), ReadChunkError::IOError(_)))
// }

#[test]
fn empty_file() {
    let bytes: Vec<u8> = vec![];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(res.unwrap_err(), ReadChunkError::EOFError));
}

#[test]
fn too_short_file() {
    let bytes: Vec<u8> = vec![b'X'];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(res.unwrap_err(), ReadChunkError::EOFError));
}

#[test]
fn no_magic_number() {
    let bytes: Vec<u8> = vec![b'X', b'D', b'A', b':'];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(res.unwrap_err(), ReadChunkError::NoMagicNumberError));
}

#[test]
fn invalid_tags() {
    //tag 0 is invalid
    let bytes: Vec<u8> = vec![b'X', b'D', b'F', b':', 1, 3, 0, 0, 10];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(res.unwrap_err(), ReadChunkError::InvalidTagError(s) if s == 0));

    //tag 7 is invalid
    let bytes: Vec<u8> = vec![b'X', b'D', b'F', b':', 1, 3, 7, 0, 10];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(res.unwrap_err(), ReadChunkError::InvalidTagError(s) if s == 7));
}


