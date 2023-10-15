// #![forbid(unsafe_code)]
#![deny(nonstandard_style)]
#![warn(array_into_iter)]
// #![warn(missing_docs)]
#![crate_type = "lib"]

//! Read XDF files
//!
//! This library provides a way to read [`XDF files`] which are up to the SCCN specifications.
//!
//! [`XDF files`]: https://github.com/sccn/xdf/wiki/Specifications

// TODO remove unused imports

use std::{collections::HashMap, rc::Rc};

mod chunk_structs;
mod errors;
mod raw_chunks;
mod streams;
mod util;

use byteorder::{ByteOrder, LittleEndian};
use chunk_structs::*;
use errors::ParseChunkError;
use raw_chunks::*;
use streams::Stream;
use util::*;
use xmltree::Element;

use crate::chunk_structs::Chunk;
use crate::raw_chunks::*;

// xdf file struct
#[derive(Debug)]
pub struct XDFFile {
    pub header: xmltree::Element,
    pub streams: HashMap<u32, Stream>,
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    String,
}

impl Format {
    const fn byte_size(self) -> usize {
        match self {
            Format::Int8 => 1,
            Format::Int16 => 2,
            Format::Int32 => 4,
            Format::Int64 => 8,
            Format::Float32 => 4,
            Format::Float64 => 8,
            Format::String => panic!("String format has no constant size"),
        }
    }
}

//This is a little annoying. Do I remove the channel_format and Fomat struct
//above entirely and just use the type of the sample's vector elements?
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
}
#[derive(Debug, PartialEq)]
pub struct Sample {
    pub timestamp: Option<f64>,
    pub values: Vec<Value>,
}

impl XDFFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::errors::Error> {
        let raw_chunks = read_to_raw_chunks(bytes)?;


            //map channel IDs to format and channel counts from streamheader chunks to
            //be able to parse sampole chunks
            let mut stream_info_map = HashMap::<u32, StreamHeaderChunkInfo>::new();
            let mut stream_num_samples_map = HashMap::<u32, u64>::new();
            let mut file_header_chunk: Option<FileHeaderChunk> = None;
            let mut stream_header_chunks: Vec<StreamHeaderChunk> = Vec::new();
            let mut stream_footer_chunks: Vec<StreamFooterChunk> = Vec::new();

            let mut sample_map = raw_chunks
                .into_iter()
                .map(|raw_chunk: RawChunk| -> Result<Chunk, crate::errors::Error> {
                    //stream IDs are always the first 4 bytes.
                    //if the chunk does not have a stream ID we can just ignore these. All
                    //chunk content bytes are longer than 4 bytes anyway.
                    let id_bytes = &raw_chunk.content_bytes[..4];
                    let stream_id: u32 = LittleEndian::read_u32(id_bytes);
                    match raw_chunk.tag {
                        Tag::FileHeader => {
                            let root = {
                                match Element::parse(raw_chunk.content_bytes.as_slice()) {
                                    Ok(root) => root,
                                    Err(err) => Err(ParseChunkError::XMLParseError(err))?,
                                }
                            };

                            let file_header_chunk = FileHeaderChunk {
                                version: parse_version(&root)?,
                                xml: root,
                            };

                            if file_header_chunk.version != 1.0 {
                                return Err(ParseChunkError::VersionNotSupportedError(file_header_chunk.version).into());
                            }

                            Ok(Chunk::FileHeaderChunk(file_header_chunk))
                        }
                        Tag::StreamHeader => parse_stream_header(&raw_chunk, &mut stream_info_map),
                        Tag::Samples => {
                            parse_samples(raw_chunk, &mut stream_num_samples_map, stream_id, &stream_info_map)
                        }
                        Tag::ClockOffset => {
                            let collection_time: f64 =
                                f64::from_le_bytes((&raw_chunk.content_bytes[4..12]).try_into()?);
                            let offset_value: f64 = f64::from_le_bytes((&raw_chunk.content_bytes[12..20]).try_into()?);

                            let clock_offset_chunk = Chunk::ClockOffsetChunk(ClockOffsetChunk {
                                stream_id,
                                collection_time,
                                offset_value,
                            });
                            Ok(clock_offset_chunk)
                        }
                        Tag::Boundary => Ok(Chunk::BoundaryChunk(BoundaryChunk {})),
                        Tag::StreamFooter => parse_stream_footer(raw_chunk, &stream_num_samples_map, &stream_info_map),
                    }
                }).filter_map(|chunk_res| {
                    let chunk = match chunk_res {
                        Ok(it) => it,
                        Err(err) => return None, // TODO ignore error?
                    };
                    match chunk {
                        Chunk::FileHeaderChunk(c) => {
                            file_header_chunk = Some(c);
                            None
                        }
                        Chunk::StreamHeaderChunk(c) => {
                            stream_header_chunks.push(c);
                            None
                        }
                        Chunk::StreamFooterChunk(c) => {
                            stream_footer_chunks.push(c);
                            None
                        }
                        Chunk::SamplesChunk(c) => {
                            Some(c)
                        }
                        _ => {None} // TODO handle clock offsets. Boundary chunks? 
                    }
                
                }).fold(HashMap::new(), |mut map, mut chunk| {
                    map.entry(chunk.stream_id).or_insert(Vec::new()).append(&mut chunk.samples);
                    map
                });



        let file_header_xml: xmltree::Element = if let Some(c) = file_header_chunk {
            c.xml.clone()
        } else {
            return Err(crate::errors::ErrorKind::MissingFileHeaderChunk.into());
        };

        let streams_res: Result<HashMap<u32, Stream>, crate::errors::Error> = {
            let stream_header_map: HashMap<u32, StreamHeaderChunk> =
                stream_header_chunks.into_iter().map(|s| (s.stream_id, s)).collect();

            let stream_footer_map: HashMap<u32, StreamFooterChunk> =
                stream_footer_chunks.into_iter().map(|s| (s.stream_id, s)).collect();

            //check if all stream headers have a corresponding stream footer
            for (&stream_id, _) in &stream_header_map {
                let _ = stream_footer_map
                    .get(&stream_id)
                    .ok_or(errors::ErrorKind::MissingStreamFooterChunk(stream_id))?;
            }

            for (&stream_id, _) in &stream_footer_map {
                let _ = stream_header_map
                    .get(&stream_id)
                    .ok_or(errors::ErrorKind::MissingStreamHeaderChunk(stream_id))?;
            }

            let mut streams_map: HashMap<u32, Stream> = HashMap::new();

            for (&stream_id, stream_header) in &stream_header_map {
                let stream_footer = stream_footer_map.get(&stream_id).unwrap();

                let name = if let Some(name) = &stream_header.info.name {
                    Some(Rc::from(name.as_str()))
                } else {
                    None
                };

                let r#type = if let Some(r#type) = &stream_header.info.r#type {
                    Some(Rc::from(r#type.as_str()))
                } else {
                    None
                };
                
                let mut most_recent_timestamp = None;
                let samples_vec =
                sample_map.remove(&stream_id)
                .unwrap_or_default() // stream could have no samples
                .into_iter()
                .enumerate()
                .map(|(i, s)| {
                    if let Some(srate) = stream_header.info.nominal_srate {
                        let timestamp = if let Some(timestamp) = s.timestamp {
                            most_recent_timestamp = Some((i, timestamp));
                            s.timestamp
                        } else {
                            let (old_i, old_timestamp) = most_recent_timestamp.unwrap();
                            Some(old_timestamp + ((i - old_i) as f64 / srate))
                        };

                        Sample {
                            timestamp,
                            values: s.values,
                        }
                    } else {
                        s
                    }
                })
                .collect();


                let stream = Stream {
                    stream_id,
                    channel_count: stream_header.info.channel_count,
                    nominal_srate: stream_header.info.nominal_srate,
                    format: stream_header.info.channel_format,

                    name,
                    r#type,
                    stream_header: stream_header.xml.clone(),
                    stream_footer: stream_footer.xml.clone(),
                    samples: samples_vec,
                };

                streams_map.insert(stream_id, stream);
            }

            Ok(streams_map)
        };

        let streams: HashMap<u32, Stream> = streams_res?;

        Ok(Self {
            header: file_header_xml,
            streams,
        })
    }
}
