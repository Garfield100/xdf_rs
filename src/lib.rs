#![forbid(unsafe_code)]
#![deny(nonstandard_style)]
#![warn(array_into_iter)]
#![crate_type = "lib"]

use byteorder::{ByteOrder, LittleEndian};
use thiserror::Error;
use xmltree::Element;

use std::{
    borrow::Cow,
    error::Error,
    fmt::Display,
    fs,
    io::{self, Read},
    path::Path,
    str::Utf8Error,
};

mod chunk_structs;
use crate::chunk_structs::*;

#[derive(Debug, Error)]
pub enum ReadChunkError {
    #[error("Could not parse file: {0}")]
    ParseError(String),

    #[error(transparent)]
    IOError(#[from] io::Error),
}

pub const FILE_TOO_SHORT_MSG: &str = "File is too short to be valid";
pub const NO_MAGIC_NUMBER_MSG: &str = "File does not begin with magic number";
pub const EARLY_EOF: &str = "Reached EOF early";

pub fn read_file_to_raw_chunks<P: AsRef<Path>>(path: P) -> Result<Vec<RawChunk>, ReadChunkError> {
    let file_bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => return Err(ReadChunkError::IOError(err)),
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

                chunk_length = match num_length_bytes.1 {
                    4 => LittleEndian::read_u32(&bytes) as u64,
                    8 => LittleEndian::read_u64(&bytes),
                    _ => unreachable!(),
                }
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

        raw_chunks.push(raw_chunk);
    }

    return Ok(raw_chunks);
}

#[derive(Debug)]
pub enum Chunk<'a> {
    FileHeaderChunk(FileHeaderChunk),
    StreamHeaderChunk(StreamHeaderChunk),
    SamplesChunk(SamplesChunk<'a>),
    ClockOffsetChunk(ClockOffsetChunk),
    BoundaryChunk(BoundaryChunk),
    StreamFooterChunk(StreamFooterChunk),
}

#[derive(Debug, Error)]
pub enum ParseChunkError {
    #[error(transparent)]
    XMLParseError(#[from] xmltree::ParseError),
    #[error("The XML tag {0} either does not exist or contains invalid or no data")]
    MissingElementError(String),
    #[error("Version {0} is not supported")]
    VersionNotSupportedError(f32),
}

fn parse_version(root: &Element) -> Result<f32, ParseChunkError> {
    let version_element = match root.get_child("version") {
        Some(child) => child,

        //XML does not contain the tag "version"
        None => return Err(ParseChunkError::MissingElementError("version".to_owned())),
    };

    let version_str = {
        match version_element.get_text() {
            Some(val) => val,

            //the version tag exists but it is empty
            None => return Err(ParseChunkError::MissingElementError("version".to_owned())),
        }
    };

    let version = {
        match version_str.parse::<f32>() {
            Ok(t) => t,

            //the version text could not be parsed into a float
            Err(e) => {
                return Err(ParseChunkError::MissingElementError("version".to_owned()));
            }
        }
    };

    if version != 1.0 {
        return Err(ParseChunkError::VersionNotSupportedError(version));
    }

    return Ok(version);
}

fn get_text_from_child(root: &Element, child_name: &str) -> Result<String, ParseChunkError> {
    Ok(root
        .get_child(child_name)
        .ok_or(ParseChunkError::MissingElementError(child_name.to_string()))?
        .get_text()
        .ok_or(ParseChunkError::MissingElementError(child_name.to_string()))?
        .to_string())
}

pub fn raw_chunk_to_chunk<'a>(raw_chunk: RawChunk) -> Result<Chunk<'a>, ParseChunkError> {
    match raw_chunk.tag {
        Tag::FileHeader => {
            let root = {
                match Element::parse(raw_chunk.content_bytes.as_slice()) {
                    Ok(root) => root,
                    Err(err) => return Err(ParseChunkError::XMLParseError(err)),
                }
            };

            return Ok(Chunk::FileHeaderChunk(FileHeaderChunk {
                version: parse_version(&root)?,
                xml: root,
            }));
        }

        Tag::StreamHeader => {
            //first 4 bytes are stream id
            let id_bytes = &raw_chunk.content_bytes[..4];
            let stream_id: u32 = LittleEndian::read_u32(id_bytes);

            let root = {
                match Element::parse(&raw_chunk.content_bytes[4..]) {
                    Ok(root) => root,
                    Err(err) => return Err(ParseChunkError::XMLParseError(err)),
                }
            };

            let version = parse_version(&root)?;

            let info = StreamHeaderChunkInfo {
                name: get_text_from_child(&root, "name")?,
                r#type: get_text_from_child(&root, "type")?,
                channel_count: get_text_from_child(&root, "channel_count")?
                    .parse()
                    .map_err(|err| {
                        ParseChunkError::MissingElementError(format!(
                            "Error while parsing channel count: {}",
                            err
                        ))
                    })?,
                nominal_srate: get_text_from_child(&root, "nominal_srate")?
                    .parse()
                    .map_err(|err| {
                        ParseChunkError::MissingElementError(format!(
                            "Error while parsing channel count: {}",
                            err
                        ))
                    })?,
                channel_format: match get_text_from_child(&root, "channel_format")?
                    .to_lowercase()
                    .as_str()
                {
                    "in8" => Format::Int8,
                    "in16" => Format::Int16,
                    "int32" => Format::Int32,
                    "int64" => Format::Int64,
                    "float32" => Format::Float32,
                    "float64" => Format::Float64,
                    "string" => Format::String,
                    invalid => {
                        return Err(ParseChunkError::MissingElementError(format!(
                            "Invalid stream format \"{}\"",
                            invalid
                        )))
                    }
                },
                created_at: get_text_from_child(&root, "created_at")?
                    .parse()
                    .map_err(|err| {
                        ParseChunkError::MissingElementError(format!(
                            "Error while parsing creation date (as f64): {}",
                            err
                        ))
                    })?,
                desc: Some(root.get_child("desc").unwrap().clone()),
            };

            

            return Ok(Chunk::StreamHeaderChunk(StreamHeaderChunk {
                stream_id,
                info,
                xml: root,
            }));
        }
        Tag::Samples => todo!(),
        Tag::ClockOffset => todo!(),
        Tag::Boundary => return Ok(Chunk::BoundaryChunk(BoundaryChunk {})),
        Tag::StreamFooter => todo!(),
    }
}
