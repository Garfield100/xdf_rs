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
use errors::XDFError;
use log::warn;
use streams::Stream;

use crate::chunk_structs::Chunk;

mod parsers;
use crate::parsers::xdf_file::xdf_file_parser;

type StreamID = u32;
type SampleIter = std::vec::IntoIter<Sample>;

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

// TODO use Arc<slice> instead of Vec?
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

struct GroupedChunks {
    stream_header_chunks: Vec<StreamHeaderChunk>,
    stream_footer_chunks: Vec<StreamFooterChunk>,
    clock_offsets: HashMap<StreamID, Vec<ClockOffsetChunk>>,
    sample_map: HashMap<StreamID, Vec<SampleIter>>,
}

impl XDFFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, XDFError> {
        let (input, chunks) = xdf_file_parser(bytes).map_err(|e| match e {
            // we have to map the error to use Arc instead of slice because we would otherwise need a static lifetime.
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

        let (file_header_chunk, grouped_chunks) = group_chunks(chunks)?;

        let streams = process_streams(grouped_chunks)?;

        Ok(Self {
            version: file_header_chunk.version,
            header: file_header_chunk.xml,
            streams,
        })
    }
}

fn group_chunks(chunks: Vec<Chunk>) -> Result<(FileHeaderChunk, GroupedChunks), XDFError> {
    let mut file_header_chunk: Option<FileHeaderChunk> = None;
    let mut stream_header_chunks: Vec<StreamHeaderChunk> = Vec::new();
    let mut stream_footer_chunks: Vec<StreamFooterChunk> = Vec::new();
    let mut clock_offsets: HashMap<StreamID, Vec<ClockOffsetChunk>> = HashMap::new();

    // the sample_map maps stream IDs to a vector of iterators which each iterate over one chunk's samples
    let sample_map = chunks
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
                Chunk::Boundary(_) => None, // boundary chunks are discarded for now
            }
        })
        .fold(
            // fold the samples into a map of stream IDs to a vector of iterators so we can merge them later
            HashMap::new(),
            |mut map: HashMap<StreamID, Vec<SampleIter>>, chunk| {
                map.entry(chunk.stream_id).or_default().push(chunk.samples.into_iter());
                map
            },
        );

    let file_header_chunk = file_header_chunk.ok_or(XDFError::MissingFileHeaderError)?;

    let info = GroupedChunks {
        stream_header_chunks,
        stream_footer_chunks,
        clock_offsets,
        sample_map,
    };

    // yes I return these separately. It saves me a clone. Sue me.
    Ok((file_header_chunk, info))
}

fn process_streams(mut grouped_chunks: GroupedChunks) -> Result<Vec<Stream>, XDFError> {
    let stream_header_map: HashMap<StreamID, StreamHeaderChunk> = grouped_chunks
        .stream_header_chunks
        .into_iter()
        .map(|s| (s.stream_id, s))
        .collect();

    let mut stream_footer_map: HashMap<StreamID, StreamFooterChunk> = grouped_chunks
        .stream_footer_chunks
        .into_iter()
        .map(|s| (s.stream_id, s))
        .collect();

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

    for (stream_id, stream_header) in stream_header_map {
        let stream_footer = stream_footer_map.remove(&stream_id);

        let name = stream_header.info.name.as_ref().map(|name| Arc::from(name.as_str()));

        let stream_type = stream_header
            .info
            .stream_type
            .as_ref()
            .map(|stream_type| Arc::from(stream_type.as_str()));

        let stream_offsets = grouped_chunks
            .clock_offsets
            .remove(&stream_header.stream_id)
            .unwrap_or_default();

        let samples_vec: Vec<Sample> = process_samples(
            grouped_chunks.sample_map.remove(&stream_id).unwrap_or_default(),
            stream_offsets,
            stream_header.info.nominal_srate,
        );

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
            stream_type,
            stream_header: stream_header.xml,
            stream_footer: stream_footer.map(|s| s.xml),
            measured_srate,
            samples: samples_vec,
        };

        streams_vec.push(stream);
    }

    Ok(streams_vec)
}

fn process_samples(
    sample_iterators: Vec<SampleIter>,
    stream_offsets: Vec<ClockOffsetChunk>,
    nominal_srate: Option<f64>,
) -> Vec<Sample> {
    let mut offset_index: usize = 0;

    let mut most_recent_timestamp = None;

    sample_iterators
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, s)| -> Sample {
            if let Some(srate) = nominal_srate {
                let timestamp = if let Some(timestamp) = s.timestamp {
                    most_recent_timestamp = Some((i, timestamp));
                    s.timestamp
                } else {
                    if let Some((old_i, old_timestamp)) = most_recent_timestamp {
                        Some(old_timestamp + ((i - old_i) as f64 / srate))
                    } else {
                        // first sample had no timestamp despite a nominal srate being specified
                        // so we just don´t assign any timestamp at all. In unusual cases this
                        // could result in a stream where the first few samples have no timestamp
                        // while the rest do. Short of looking ahead there isn't anything we can do.
                        None
                    }
                };

                let timestamp = timestamp.map(|ts| interpolate_and_add_offsets(ts, &stream_offsets, &mut offset_index));

                Sample {
                    timestamp,
                    values: s.values,
                }
            } else {
                s
            }
        })
        .collect()
}

fn interpolate_and_add_offsets(ts: f64, stream_offsets: &Vec<ClockOffsetChunk>, offset_index: &mut usize) -> f64 {
    if !stream_offsets.is_empty() {
        // TODO add clock offset to timestamp

        let time_or_nan = |i: usize| {
            stream_offsets
                .get(i + 1)
                .map_or(f64::NAN, |c: &ClockOffsetChunk| c.collection_time)
            //use NaN to break out of the loop below in case we've gone out of bounds
            // this avoids an infinite loop in the unusual case where all clock offsets are newer than the timestamp.
        };

        // if the current timestamp is older than the what the current offset would imply,
        // the offset must either be zero (and the timestamp older than *every* offset),
        // or something has gone horribly wrong.

        // indexing to zero is safe because we know the vector is not empty
        if ts < stream_offsets[0].collection_time {
            assert_eq!(
                *offset_index, 0,
                "Timestamp is older than the first clock offset, but the offset index is not zero."
            );
            return ts + stream_offsets[0].offset_value;
        }

        // ensure clock offset at offset_index is older than the current timestamp
        while ts > time_or_nan(*offset_index) {
            *offset_index += 1;
        }

        // TODO verify this somehow
        // get the most recent offset before the current timestamp
        let prev_offset = stream_offsets.get(*offset_index).or_else(|| stream_offsets.last());

        // and the clock offset which comes next
        let next_offset = stream_offsets.get(*offset_index + 1).or_else(|| stream_offsets.last());

        let interpolated = if let (Some(l), Some(n)) = (prev_offset, next_offset) {
            // nearly all cases will have to be interpolated
            // a * (1-x) + b * x (with x between 0 and 1 of course)

            let dt = n.collection_time - l.collection_time;

            // can be zero if the offsets are the same
            if dt > 0.0 {
                let t_normalised = (ts - l.collection_time) / dt;
                l.offset_value * (1.0 - t_normalised) + n.offset_value * t_normalised
            } else {
                l.offset_value
            }
        } else {
            prev_offset.or(next_offset).map_or(0.0, |c| c.offset_value)
        };

        ts + interpolated
    } else {
        ts //there are no offsets
    }
}

// TESTS

#[cfg(test)]
const EPSILON: f64 = 1E-15;

// test the interpolation function for timestamps *inside* the range of offsets
#[test]
fn test_interpolation_inside() {
    let offsets = vec![
        ClockOffsetChunk {
            collection_time: 0.0,
            offset_value: -1.0,
            stream_id: 0,
        },
        ClockOffsetChunk {
            collection_time: 1.0,
            offset_value: 1.0,
            stream_id: 0,
        },
    ];

    // test at multiple steps
    for i in 0..=10 {
        let timestamp = i as f64 / 10.0;

        let mut offset_index = 0;
        let interpolated = interpolate_and_add_offsets(timestamp, &offsets, &mut offset_index);

        let expected = timestamp + (timestamp * 2.0 - 1.0); // original timestamp + interpolated offset

        assert!(
            (interpolated - expected).abs() < EPSILON,
            "expected {} to be within {} of {}",
            interpolated,
            EPSILON,
            expected
        );
    }
}

// test the interpolation function for timestamps after the last offset
#[test]
fn test_interpolation_after() {
    let offsets = vec![
        ClockOffsetChunk {
            collection_time: 0.0,
            offset_value: -1.0,
            stream_id: 0,
        },
        ClockOffsetChunk {
            collection_time: 1.0,
            offset_value: 1.0,
            stream_id: 0,
        },
    ];
    // after the range we expect for the last offset to be used
    let last_offset = offsets.last().unwrap();
    let timestamp = last_offset.collection_time + 1.0;
    let mut offset_index = 0;
    let interpolated = interpolate_and_add_offsets(timestamp, &offsets, &mut offset_index);
    let expected = timestamp + last_offset.offset_value;

    assert!(
        (interpolated - expected).abs() < EPSILON,
        "expected {} to be within {} of {}",
        interpolated,
        EPSILON,
        expected
    );
}

// test the interpolation function for timestamps before the first offset
#[test]
fn test_interpolation_before() {
    let offsets = vec![
        ClockOffsetChunk {
            collection_time: 0.0,
            offset_value: -1.0,
            stream_id: 0,
        },
        ClockOffsetChunk {
            collection_time: 1.0,
            offset_value: 1.0,
            stream_id: 0,
        },
    ];
    // after the range we expect for the last offset to be used
    let first_offset = offsets.first().unwrap();
    let timestamp = first_offset.collection_time - 1.0;
    let mut offset_index = 0;
    let interpolated = interpolate_and_add_offsets(timestamp, &offsets, &mut offset_index);
    let expected = timestamp + first_offset.offset_value;

    assert!(
        (interpolated - expected).abs() < EPSILON,
        "expected {} to be within {} of {}",
        interpolated,
        EPSILON,
        expected
    );
}

// Make sure a bad offset fails the assetion in the function. More details within the tested function.
#[test]
#[should_panic]
fn test_interpolation_bad_offset() {
    let offsets = vec![
        ClockOffsetChunk {
            collection_time: 0.0,
            offset_value: -1.0,
            stream_id: 0,
        },
        ClockOffsetChunk {
            collection_time: 1.0,
            offset_value: 1.0,
            stream_id: 0,
        },
    ];
    // after the range we expect for the last offset to be used
    let first_offset = offsets.first().unwrap();
    let timestamp = first_offset.collection_time - 1.0;
    let mut offset_index = 1;

    // should panic
    interpolate_and_add_offsets(timestamp, &offsets, &mut offset_index);
}
