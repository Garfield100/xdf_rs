#![forbid(unsafe_code)]
#![deny(nonstandard_style)]
#![warn(array_into_iter)]
// #![warn(missing_docs)]
#![crate_type = "lib"]

//! [![github]](https://github.com/Garfield100/xdf_rs)
//!
//! [github]: https://img.shields.io/badge/github-9090ff?style=for-the-badge&logo=github&labelColor=505050
//!

//! Read XDF files
//!
//! [`XDF format`]: https://github.com/sccn/xdf/wiki/Specifications
//!
//! This library provides a way to read files in the [`XDF format`] as specified by SCCN.
//!

use std::sync::Arc;
use std::{collections::HashMap, rc::Rc};

mod chunk_structs;
mod errors;

mod streams;
mod util;

use chunk_structs::*;
use errors::XdfError;
use log::warn;
use streams::Stream;

use crate::chunk_structs::Chunk;

pub(crate) mod parsers;
use crate::parsers::chunk_parsers::chunk_root;

type StreamID = u32;
// xdf file struct
#[derive(Debug)]
pub struct XDFFile {
    pub version: f32,
    pub header: xmltree::Element,
    pub streams: Vec<Stream>,
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
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, XdfError> {
        let (input, chunks) = chunk_root(bytes).map_err(|e| match e {
            // we have to map the error to use Arc instead of slice because error_chain doesn't like lifetimes in external errors
            nom::Err::Incomplete(n) => nom::Err::Incomplete(n),
            nom::Err::Error(nom::error::Error { input, code }) => nom::Err::Error(nom::error::Error {
                input: Arc::from(input.to_owned()),
                code,
            }),
            nom::Err::Failure(nom::error::Error { input, code }) => nom::Err::Failure(nom::error::Error {
                input: Arc::from(input.to_owned()),
                code,
            }),
        })?;

        if !input.is_empty() {
            warn!("There are {} bytes left in the input after parsing.", input.len());
        }

        let mut file_header_chunk: Option<FileHeaderChunk> = None;
        let mut stream_header_chunks: Vec<StreamHeaderChunk> = Vec::new();
        let mut stream_footer_chunks: Vec<StreamFooterChunk> = Vec::new();
        let mut clock_offsets = HashMap::<StreamID, Vec<ClockOffsetChunk>>::new();

        // the sample_map maps stream IDs to a vector of iterators which each iterate over one chunk's samples
        let mut sample_map = chunks
            .into_iter()
            .filter_map(|chunk_res| {
                match chunk_res {
                    Chunk::FileHeader(c) => {
                        file_header_chunk = Some(c);
                        None
                    }
                    Chunk::StreamHeader(c) => {
                        stream_header_chunks.push(c);
                        None
                    }
                    Chunk::StreamFooter(c) => {
                        stream_footer_chunks.push(c);
                        None
                    }
                    Chunk::Samples(c) => Some(c), // pass only samples through to the fold
                    Chunk::ClockOffset(c) => {
                        clock_offsets.entry(c.stream_id).or_default().push(c);

                        None
                    }
                    Chunk::Boundary(_) => None,
                }
            })
            .fold(
                // fold the samples into a map of stream IDs to a vector of iterators so we can merge them later
                HashMap::new(),
                |mut map: HashMap<StreamID, Vec<std::vec::IntoIter<Sample>>>, chunk| {
                    map.entry(chunk.stream_id).or_default().push(chunk.samples.into_iter());
                    map
                },
            );

        let version;
        let file_header_xml: xmltree::Element = if let Some(c) = file_header_chunk {
            version = c.version;
            c.xml
        } else {
            return Err(XdfError::MissingFileHeaderError); // this should already be covered by the nom parser
        };

        let streams_res: Result<Vec<Stream>, XdfError> = {
            let stream_header_map: HashMap<StreamID, StreamHeaderChunk> =
                stream_header_chunks.into_iter().map(|s| (s.stream_id, s)).collect();

            let stream_footer_map: HashMap<StreamID, StreamFooterChunk> =
                stream_footer_chunks.into_iter().map(|s| (s.stream_id, s)).collect();

            // this can happen if the recording stops unexpectedly. We allow this to be more error tolerant and not lose all experimental data.
            for &stream_id in stream_header_map.keys() {
                if let None = stream_footer_map.get(&stream_id) {
                    warn!(
                        "Stream header without corresponding stream footer for id: {}",
                        stream_id
                    );
                }
            }

            // this on the other hand is a bit weirder but again, we allow it to be more error tolerant
            for &stream_id in stream_footer_map.keys() {
                if let None = stream_header_map.get(&stream_id) {
                    warn!(
                        "Stream footer without corresponding stream header for id: {}",
                        stream_id
                    );
                }
            }

            let mut streams_vec: Vec<Stream> = Vec::new();

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
                    id: stream_id,
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

                streams_vec.push(stream);
            }

            Ok(streams_vec)
        };

        let streams = streams_res?;

        Ok(Self {
            version,
            header: file_header_xml,
            streams,
        })
    }
}
