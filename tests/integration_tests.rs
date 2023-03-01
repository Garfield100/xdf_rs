use xdf::{read_file_to_raw_chunks, ReadChunkError, FILE_TOO_SHORT_MSG, NO_MAGIC_NUMBER_MSG};
use assert_fs::{prelude::*, TempDir};

#[test]
fn invalid_path() {
    let res = read_file_to_raw_chunks("./does/not/exist.xdf");
    assert!(matches!(res.unwrap_err(), ReadChunkError::IOError(_)))
}

#[test]
fn empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let empty_file = temp_dir.child("empty.xdf");
    empty_file.touch().unwrap();

    let res = read_file_to_raw_chunks(empty_file.path());
    assert!(
        matches!(res.unwrap_err(), ReadChunkError::ParseError(s) if s == FILE_TOO_SHORT_MSG.to_string() )
    );
}

#[test]
fn no_magic_number() {
    let temp_dir = TempDir::new().unwrap();
    let no_magic_file = temp_dir.child("no_magic.xdf");
    no_magic_file.touch().unwrap();
    no_magic_file.write_str("NOT: a magic number").unwrap();

    let res = read_file_to_raw_chunks(no_magic_file.path());
    assert!(
        matches!(res.unwrap_err(), ReadChunkError::ParseError(s) if s == NO_MAGIC_NUMBER_MSG.to_string() )
    );
}

