#![forbid(unsafe_code)]
#![deny(nonstandard_style)]
#![warn(array_into_iter)]
#![crate_type = "lib"]

use byteorder::{ByteOrder, LittleEndian};
use thiserror::Error;
use xmltree::Element;

use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    fmt::{format, Display},
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
            1 => chunk_length = *content_iter.next().unwrap().1 as u64,
            4 | 8 => {
                let mut bytes: Vec<u8> = vec![0; *num_length_bytes.1 as usize];
                for i in 0..bytes.len() {
                    if let Some(next_byte) = content_iter.next() {
                        bytes[i] = *next_byte.1;
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
                    return Err(ReadChunkError::ParseError(format!("More than one FileHeaders found.")));
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
                    Some(val) => *val.1,
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
pub enum Chunk {
    FileHeaderChunk(FileHeaderChunk),
    StreamHeaderChunk(StreamHeaderChunk),
    SamplesChunk(SamplesChunk),
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
    #[error("Other error. Reason: {0}")]
    Other(String),
}

fn parse_version(root: &Element) -> Result<f32, ParseChunkError> {
    let version_element = match root.get_child("version") {
        Some(child) => child,

        //XML does not contain the tag "version"
        None => return Err(ParseChunkError::MissingElementError("version".to_string())),
    };

    let version_str = {
        match version_element.get_text() {
            Some(val) => val,

            //the version tag exists but it is empty
            None => return Err(ParseChunkError::MissingElementError("version".to_string())),
        }
    };

    let version = {
        match version_str.parse::<f32>() {
            Ok(t) => t,

            //TODO improve this error handling
            //the version text could not be parsed into a float
            Err(e) => {
                return Err(ParseChunkError::MissingElementError("version".to_string()));
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

pub fn raw_chunks_to_chunks(raw_chunks: Vec<RawChunk>) -> Result<Vec<Chunk>, ParseChunkError> {
    let mut chunks: Vec<Chunk> = vec![];

    //map channel IDs to format and channel counts from streamheader chunks to
    //be able to parse sampole chunks
    let mut stream_info_map = HashMap::<u32, (Format, u32)>::new();

    for raw_chunk in raw_chunks {
        //stream IDs are always the first 4 bytes.
        //if the chunk does not have a stream ID we can just ignore these. All
        //chunk content bytes are longer than 4 bytes anyway.
        let id_bytes = &raw_chunk.content_bytes[..4];
        let stream_id: u32 = LittleEndian::read_u32(id_bytes);
        println!("{:?}", raw_chunk.tag);
        match raw_chunk.tag {
            Tag::FileHeader => {
                let root = {
                    match Element::parse(raw_chunk.content_bytes.as_slice()) {
                        Ok(root) => root,
                        Err(err) => return Err(ParseChunkError::XMLParseError(err)),
                    }
                };
                chunks.push(Chunk::FileHeaderChunk(FileHeaderChunk {
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

                let info = StreamHeaderChunkInfo {
                    name: get_text_from_child(&root, "name")?,
                    r#type: get_text_from_child(&root, "type")?,
                    channel_count: get_text_from_child(&root, "channel_count")?.parse().map_err(|err| {
                        ParseChunkError::MissingElementError(format!("Error while parsing channel count: {}", err))
                    })?,
                    nominal_srate: get_text_from_child(&root, "nominal_srate")?.parse().map_err(|err| {
                        ParseChunkError::MissingElementError(format!("Error while parsing channel count: {}", err))
                    })?,
                    channel_format: match get_text_from_child(&root, "channel_format")?.to_lowercase().as_str() {
                        "in8" => Format::Int8,
                        "int16" => Format::Int16,
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
                    created_at: get_text_from_child(&root, "created_at")?.parse().map_err(|err| {
                        ParseChunkError::MissingElementError(format!(
                            "Error while parsing creation date (as f64): {}",
                            err
                        ))
                    })?,
                    desc: Some(root.get_child("desc").unwrap().clone()),
                };

                stream_info_map.insert(stream_id, (info.channel_format, info.channel_count));

                let stream_header_chunk = StreamHeaderChunk {
                    stream_id,
                    info,
                    xml: root,
                };

                chunks.push(Chunk::StreamHeaderChunk(stream_header_chunk));
            }
            Tag::Samples => {
                //number of bytes used to represent the number of samples contained
                //in this chunk
                let num_samples_byte_num = &raw_chunk.content_bytes[4];
                println!("Sample chunk bytes:\n{:?}", raw_chunk.content_bytes);
                println!("Sample chunk length: {}", raw_chunk.content_bytes.len());
                println!("Sample chunk stream id: {}", stream_id);

                //allow only valid options as per spec
                match num_samples_byte_num {
                    1 | 4 | 8 => (),
                    _ => {
                        return Err(ParseChunkError::Other(format!(
                            "Invalid amount of sample number bytes: was {} but expected 1, 4, or 8.",
                            num_samples_byte_num
                        )))
                    }
                }

                //vector of bytes which together form the number of samples
                let num_samples_bytes = &raw_chunk.content_bytes[5..(5 + num_samples_byte_num) as usize];

                //the actual number of samples
                let num_samples: u64 = LittleEndian::read_uint(num_samples_bytes, *num_samples_byte_num as usize);

                //TODO: bounds checks. probably use .get or something

                //for numeric values:
                //number of values = no. channels
                //size of a sample = 1 + (0 or 8) + no. channels * size of type
                //fuck, the timestamp thing makes it variable
                //why
                //realistically there will be timestamps for either all samples or
                //for none of them but the spec technically allows for other stuff
                //ffs

                let (sample_format, channel_count) = stream_info_map.get(&stream_id).ok_or(ParseChunkError::Other(format!("Chunks in file are out of order or otherwise invalid: could not find stream header chunk for stream id {}", stream_id)))?;

                //option here because string doesn't have a constant size
                let type_size: Option<i32> = match sample_format {
                    Format::Int8 => Some(1),
                    Format::Int16 => Some(2),
                    Format::Int32 => Some(4),
                    Format::Int64 => Some(8),
                    Format::Float32 => Some(4),
                    Format::Float64 => Some(8),
                    Format::String => None,
                };

                //offset:
                // 4 bytes for streamID
                // 1 byte for number of bytes for sample count
                // num_samples_byte_num for sample count
                let mut offset: usize = 4 + 1 + *num_samples_byte_num as usize;

                let mut samples: Vec<Sample> = Vec::with_capacity(num_samples as usize);

                //TODO is it worth having two loops only to not have to check
                //inside?
                //pro: performance? should test
                //cons: duplicate code
                if let Some(type_size) = type_size {
                    //constant size types
                    for _ in 0..num_samples {
                        let mut values: Vec<Value> = Vec::with_capacity(*channel_count as usize);
                        let timestamp: Option<f64> = extract_timestamp(&raw_chunk, &mut offset);

                        //values
                        for _ in 0..*channel_count {
                            let value_bytes = &raw_chunk.content_bytes[offset..(offset + type_size as usize)];
                            let value: Value = match sample_format {
                                Format::Int8 => Value::Int8(value_bytes[0] as i8),
                                Format::Int16 => Value::Int16(LittleEndian::read_i16(value_bytes)),
                                Format::Int32 => Value::Int32(LittleEndian::read_i32(value_bytes)),
                                Format::Int64 => Value::Int64(LittleEndian::read_i64(value_bytes)),
                                Format::Float32 => Value::Float32(LittleEndian::read_f32(value_bytes)),
                                Format::Float64 => Value::Float64(LittleEndian::read_f64(value_bytes)),
                                Format::String => unreachable!(),
                            };

                            values.push(value);
                            offset += type_size as usize;
                        }

                        println!("Values: {:?}", &values);
                        samples.push(Sample { timestamp, values });
                    }
                } else {
                    let mut values: Vec<Value> = vec![];
                    let timestamp: Option<f64> = extract_timestamp(&raw_chunk, &mut offset);
                    //strings
                    for _ in 0..num_samples {
                        let num_length_bytes: usize = raw_chunk.content_bytes[offset] as usize;
                        offset += 1; //for number of length bytes field
                        let value_length = match num_length_bytes {
                            1 | 4 | 8 => LittleEndian::read_uint(
                                &raw_chunk.content_bytes[offset..(offset + num_length_bytes)],
                                num_length_bytes,
                            ),
                            _ => {
                                println!("Error: Number of length bytes for this value are invalid. Expected either 4 or 8 but got {}", num_length_bytes);
                                println!("num_length_bytes: {}", num_length_bytes);
                                println!("offset: {}", offset);
                                println!("Chunk bytes len: {}", &raw_chunk.content_bytes.len());
                                println!("Chunk bytes:\n{:?}", &raw_chunk.content_bytes);
                                panic!();
                            } //TODO error properly
                        } as usize;
                        offset += num_length_bytes; // for length field
                        let mut value_bytes = &raw_chunk.content_bytes[offset..(offset + value_length)];

                        //TODO what in the cursed and why
                        let mut value_string = String::new();
                        value_bytes.read_to_string(&mut value_string); //TODO handle utf8 err

                        println!("String value: {}", &value_string);
                        let value = Value::String(value_string);
                        values.push(value);
                        offset += value_length; // for value field
                        offset += 1; // ???
                    }
                    samples.push(Sample { timestamp, values });
                }

                let samples_chunk = Chunk::SamplesChunk(SamplesChunk { stream_id, samples });
                // println!("{:#?}", &samples_chunk);
                chunks.push(samples_chunk);
            }
            Tag::ClockOffset => todo!(),
            Tag::Boundary => chunks.push(Chunk::BoundaryChunk(BoundaryChunk {})),
            Tag::StreamFooter => todo!(),
        }
    }

    return Ok(chunks);
}

fn extract_timestamp(raw_chunk: &RawChunk, offset: &mut usize) -> Option<f64> {
    let timestamp: Option<f64>;
    if raw_chunk.content_bytes[*offset] == 8 {
        //we have a timestamp
        timestamp = Some(LittleEndian::read_f64(
            &raw_chunk.content_bytes[(*offset + 1)..(*offset + 9)],
        ));
        *offset += 9;
    } else {
        //no timestamp
        timestamp = None;
        *offset += 1;
    }

    return timestamp;
}
