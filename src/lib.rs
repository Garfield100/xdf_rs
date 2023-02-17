#![forbid(unsafe_code)]
#![warn(array_into_iter)]

use byteorder::{ByteOrder, LittleEndian};

use std::{
    error::Error,
    fmt::Display,
    fs,
    io::{self},
    path::Path,
};

#[derive(Debug)]
pub enum Tag {
    FileHeader,
    StreamHeader,
    Samples,
    ClockOffset,
    Boundary,
    StreamFooter,
}

#[derive(Debug)]
pub struct RawChunk {
    tag: Tag,
    content_bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum ReadChunkError {
    IoError(io::Error),
    ParseError(String),
}

impl Display for ReadChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadChunkError::IoError(err) => err.fmt(f),
            ReadChunkError::ParseError(msg) => write!(f, "ParseError: {}", msg),
        }
    }
}

impl Error for ReadChunkError {}

const FILE_TOO_SHORT_MSG: &str = "File is too short to be valid";
const NO_MAGIC_NUMBER_MSG: &str = "File does not begin with magic number";
const EARLY_EOF: &str = "Reached EOF early";

pub fn read_file_to_chunks<P: AsRef<Path>>(path: P) -> Result<Vec<RawChunk>, ReadChunkError> {
    let file_bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => return Err(ReadChunkError::IoError(err)),
    };

    if file_bytes.len() < "XDF:".len() {
        return Err(ReadChunkError::ParseError(FILE_TOO_SHORT_MSG.to_string()));
    }

    if &file_bytes[0..4] != "XDF:".as_bytes() {
        return Err(ReadChunkError::ParseError(NO_MAGIC_NUMBER_MSG.to_string()));
    }

    let mut raw_chunks: Vec<RawChunk> = Vec::new();
    let mut file_header_found: bool = false;

    let mut content_iter = file_bytes.iter().enumerate().peekable().skip("XDF:".len());

    while let Some(num_length_bytes) = content_iter.next() {
        let mut chunk_length: u64;
        match num_length_bytes.1 {
            1 => chunk_length = content_iter.next().unwrap().1.clone() as u64,
            4 | 8 => {
                let mut bytes: Vec<u8> = vec![0; *num_length_bytes.1 as usize];
                for i in 0..bytes.len() {
                    if let Some(next_byte) = content_iter.next() {
                        bytes[i] = next_byte.1.clone();
                    } else {
                        return Err(ReadChunkError::ParseError(EARLY_EOF.to_string()));
                    }
                }
                chunk_length = LittleEndian::read_u32(&bytes) as u64;
            }

            _ => {
                return Err(ReadChunkError::ParseError(format!(
                    "Invalid number of chunk length bytes found at index {}. Expected 1, 4, or 8 but was {}",
                    num_length_bytes.0, num_length_bytes.1
                )));
            }
        }

        let mut tag_bytes: [u8; 2] = [0; 2];
        for i in 0..tag_bytes.len() {
            tag_bytes[i] = {
                let val = content_iter.next();
                match val {
                    Some(val) => val.1,
                    None => return Err(ReadChunkError::ParseError(EARLY_EOF.to_string())),
                }
            }
            .clone();
        }

        let chunk_tag_num = LittleEndian::read_u16(&tag_bytes);

        let chunk_tag: Tag = match chunk_tag_num {
            1 => {
                if file_header_found {
                    return Err(ReadChunkError::ParseError(format!(
                        "More than one FileHeaders found."
                    )));
                }
                file_header_found = true;
                Tag::FileHeader
            }
            2 => Tag::StreamHeader,
            3 => Tag::Samples,
            4 => Tag::ClockOffset,
            5 => Tag::Boundary,
            6 => Tag::StreamFooter,
            _ => {
                return Err(ReadChunkError::ParseError(format!(
                    "Invalid tag. Expected 1 to 6 inclusive but was {}",
                    chunk_tag_num
                )))
            }
        };

        //subtract the two tag bytes for the content length
        chunk_length -= 2;

        // try to cast the chunk length to usize in order to allocate a vector with it
        let chunk_length: usize = match (chunk_length).try_into() {
            Ok(len) => len,
            Err(err) => {
                return Err(ReadChunkError::ParseError(format!(
                    "Chunk too big. Cannot cast {} to usize\n{}",
                    chunk_length, err
                )));
            }
        };

        let mut chunk_bytes: Vec<u8> = vec![0; chunk_length];
        for i in 0..chunk_length {
            chunk_bytes[i] = {
                match content_iter.next() {
                    Some(val) => val.1.clone(),
                    None => return Err(ReadChunkError::ParseError(EARLY_EOF.to_string())),
                }
            };
        }

        let raw_chunk = RawChunk {
            tag: chunk_tag,
            content_bytes: chunk_bytes,
        };
        println!("{:#?}", &raw_chunk);
        println!("{}", chunk_length);
        raw_chunks.push(raw_chunk);
    }

    return Ok(raw_chunks);
}

#[cfg(test)]
mod tests {
    use crate::{read_file_to_chunks, ReadChunkError, FILE_TOO_SHORT_MSG, NO_MAGIC_NUMBER_MSG};
    use assert_fs::{prelude::*, TempDir};

    #[test]
    fn invalid_path() {
        let res = read_file_to_chunks("./does/not/exist.xdf");
        assert!(matches!(res.unwrap_err(), ReadChunkError::IoError(_)))
    }

    #[test]
    fn empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.child("empty.xdf");
        empty_file.touch().unwrap();

        let res = read_file_to_chunks(empty_file.path());
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

        let res = read_file_to_chunks(no_magic_file.path());
        assert!(
            matches!(res.unwrap_err(), ReadChunkError::ParseError(s) if s == NO_MAGIC_NUMBER_MSG.to_string() )
        );
    }
}
