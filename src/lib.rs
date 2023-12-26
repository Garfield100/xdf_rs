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

use std::{collections::HashMap, rc::Rc};

mod chunk_structs;
mod errors;
mod raw_chunks;
mod streams;
mod util;

use byteorder::{ByteOrder, LittleEndian};
use chunk_structs::*;
use error_chain::bail;
use errors::ErrorKind;
// use errors::ParseChunkError;
use raw_chunks::*;
use streams::Stream;
use util::*;
use xmltree::Element;

use crate::chunk_structs::Chunk;

type StreamID = u32;
// xdf file struct
#[derive(Debug)]
pub struct XDFFile {
    pub version: f32,
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

// TODO use Rc<slice> instead of Vec?
#[derive(Debug, Clone, PartialEq)]
pub enum Values {
    Int8(Vec<i8>),
    Int16(Vec<i16>),
    Int32(Vec<i32>),
    Int64(Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    String(String),
}

#[derive(Debug, PartialEq)]
pub struct Sample {
    pub timestamp: Option<f64>,
    pub values: Values,
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
        let mut clock_offsets = HashMap::<StreamID, Vec<ClockOffsetChunk>>::new();

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
                        let root = Element::parse(raw_chunk.content_bytes.as_slice())?;
                        let version = parse_version(&root)?;

                        if version != 1.0 {
                            return Err(ErrorKind::VersionNotSupportedError(version).into());
                        }

                        let file_header_chunk = FileHeaderChunk {
                            version: parse_version(&root)?,
                            xml: root,
                        };

                        Ok(Chunk::FileHeader(file_header_chunk))
                    }
                    Tag::StreamHeader => parse_stream_header(&raw_chunk, &mut stream_info_map),
                    Tag::Samples => parse_samples(raw_chunk, &mut stream_num_samples_map, stream_id, &stream_info_map),
                    Tag::ClockOffset => {
                        let collection_time: f64 = f64::from_le_bytes((&raw_chunk.content_bytes[4..12]).try_into()?);
                        let offset_value: f64 = f64::from_le_bytes((&raw_chunk.content_bytes[12..20]).try_into()?);

                        let clock_offset_chunk = Chunk::ClockOffset(ClockOffsetChunk {
                            stream_id,
                            collection_time,
                            offset_value,
                        });
                        Ok(clock_offset_chunk)
                    }
                    Tag::Boundary => Ok(Chunk::Boundary(BoundaryChunk {})),
                    Tag::StreamFooter => parse_stream_footer(raw_chunk),
                }
            })
            .filter_map(|chunk_res| {
                match chunk_res {
                    Ok(Chunk::FileHeader(c)) => {
                        file_header_chunk = Some(c);
                        None
                    }
                    Ok(Chunk::StreamHeader(c)) => {
                        stream_header_chunks.push(c);
                        None
                    }
                    Ok(Chunk::StreamFooter(c)) => {
                        stream_footer_chunks.push(c);
                        None
                    }
                    Ok(Chunk::Samples(c)) => Some(c),
                    Ok(Chunk::ClockOffset(c)) => {
                        clock_offsets.entry(c.stream_id).or_default().push(c);

                        None
                    }
                    Ok(Chunk::Boundary(_)) => None,
                    Err(_err) => {
                        None // TODO log error?
                    }
                }
            })
            .fold(
                HashMap::new(),
                |mut map: HashMap<u32, Vec<std::vec::IntoIter<Sample>>>, chunk| {
                    map.entry(chunk.stream_id).or_default().push(chunk.samples.into_iter());
                    map
                },
            );

        let version;
        let file_header_xml: xmltree::Element = if let Some(c) = file_header_chunk {
            version = c.version;
            c.xml
        } else {
            bail!(ErrorKind::MissingFileHeaderError);
        };

        let streams_res: Result<HashMap<u32, Stream>, crate::errors::Error> = {
            let stream_header_map: HashMap<u32, StreamHeaderChunk> =
                stream_header_chunks.into_iter().map(|s| (s.stream_id, s)).collect();

            let stream_footer_map: HashMap<u32, StreamFooterChunk> =
                stream_footer_chunks.into_iter().map(|s| (s.stream_id, s)).collect();

            // TODO we might want to reduce this to a log warning to be more error tolerant in case a recording stopped unexpectedly
            // check if all stream headers have a corresponding stream footer
            for &stream_id in stream_footer_map.keys() {
                stream_footer_map
                    .get(&stream_id)
                    .ok_or_else(|| errors::ErrorKind::MissingStreamFooterChunk(stream_id))?;
            }

            for &stream_id in stream_footer_map.keys() {
                stream_header_map
                    .get(&stream_id)
                    .ok_or_else(|| errors::ErrorKind::MissingStreamHeaderError(stream_id))?;
            }

            let mut streams_map: HashMap<u32, Stream> = HashMap::new();

            for (&stream_id, stream_header) in &stream_header_map {
                let stream_footer = stream_footer_map.get(&stream_id);

                let name = stream_header.info.name.as_ref().map(|name| Rc::from(name.as_str()));

                let r#type = stream_header
                    .info
                    .r#type
                    .as_ref()
                    .map(|r#type| Rc::from(r#type.as_str()));

                let stream_offsets = clock_offsets.remove(&stream_id).unwrap_or_default();
                let mut offset_index: usize = 0;

                let mut most_recent_timestamp = None;
                let samples_vec: Vec<Sample> = sample_map
                    .remove(&stream_id)
                    .unwrap_or_default() // stream could have no samples
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, s)| {
                        if let Some(srate) = stream_header.info.nominal_srate {
                            let timestamp = if let Some(timestamp) = s.timestamp {
                                most_recent_timestamp = Some((i, timestamp));
                                s.timestamp
                            } else {
                                let (old_i, old_timestamp) = most_recent_timestamp.unwrap(); // TODO this panics if the first sample has no timestamp. What do?
                                Some(old_timestamp + ((i - old_i) as f64 / srate))
                            };

                            let timestamp = if let Some(ts) = timestamp {
                                if !stream_offsets.is_empty() {
                                    // TODO add clock offset to timestamp

                                    let time_or_nan = |i| {
                                        stream_offsets
                                            .get(i + 1)
                                            .map_or(f64::NAN, |c: &ClockOffsetChunk| c.collection_time)
                                        //use NaN to break out of the loop below in case we've gone out of bounds
                                        // this avoids an infinite loop in the unusual case where all clock offsets are newer than the newest timestamp.
                                    };

                                    // ensure clock offset at offset_index is older than the current timestamp
                                    while ts > time_or_nan(offset_index) {
                                        offset_index += 1;
                                    }

                                    // TODO verify this somehow
                                    // get the most recent offset before the current timestamp
                                    let last_offset =
                                        stream_offsets.get(offset_index).or_else(|| stream_offsets.last());

                                    // and the clock offset which comes next
                                    let next_offset =
                                        stream_offsets.get(offset_index + 1).or_else(|| stream_offsets.last());

                                    let interpolated = if let (Some(l), Some(n)) = (last_offset, next_offset) {
                                        // most cases will fall into this
                                        // a * (1-x) + b * x

                                        let dt = n.collection_time - l.collection_time;

                                        let t_normalised = (ts - l.collection_time) / dt;

                                        l.offset_value * (1.0 - t_normalised) + n.offset_value * t_normalised
                                    } else {
                                        last_offset.or(next_offset).map_or(0.0, |c| c.offset_value)
                                    };

                                    Some(ts + interpolated)
                                } else {
                                    timestamp //Some but there are no offsets
                                }
                            } else {
                                timestamp //None
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

                let measured_srate = if stream_header.info.nominal_srate.is_some() {
                    // nominal_srate is given as "a floating point number in Hertz. If the stream
                    // has an irregular sampling rate (that is, the samples are not spaced evenly in
                    // time, for example in an event stream), this value must be 0."
                    // we use None instead of 0.

                    let first_timestamp: Option<f64> = samples_vec.first().and_then(|s| s.timestamp);
                    let last_timestamp: Option<f64> = samples_vec.last().and_then(|s| s.timestamp);

                    if let (num_samples, Some(first_timestamp), Some(last_timestamp)) =
                        (samples_vec.len(), first_timestamp, last_timestamp)
                    {
                        if num_samples == 0 {
                            None // don't divide by zero :)
                        } else {
                            Some((last_timestamp - first_timestamp) / num_samples as f64)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let stream = Stream {
                    stream_id,
                    channel_count: stream_header.info.channel_count,
                    nominal_srate: stream_header.info.nominal_srate,
                    format: stream_header.info.channel_format,

                    name,
                    r#type,
                    stream_header: stream_header.xml.clone(),
                    stream_footer: stream_footer.map(|s| s.xml.clone()),
                    measured_srate,
                    samples: samples_vec,
                };

                streams_map.insert(stream_id, stream);
            }

            Ok(streams_map)
        };

        let streams: HashMap<u32, Stream> = streams_res?;

        Ok(Self {
            version,
            header: file_header_xml,
            streams,
        })
    }
}
