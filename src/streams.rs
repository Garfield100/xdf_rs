use std::collections::HashMap;
use std::rc::Rc;

use crate::chunk_structs::{Chunk, FileHeaderChunk, SamplesChunk, StreamFooterChunk, StreamHeaderChunk};
use crate::errors::{self, Result};
use crate::{Format, Sample};

// minimal tags in version 1.x:
// channel count
// nominal srate
// channel format

// common additional tags:
// name
// type
// desc

#[derive(Debug)]
pub struct Stream {
    pub stream_id: u32, // TODO only used internally to match stream headers, footers, and samples

    pub channel_count: u32,
    pub nominal_srate: Option<f64>, //a mandatory field but we replace zero with None
    pub format: Format,

    // optional fields
    pub name: Option<Rc<str>>,
    pub r#type: Option<Rc<str>>,

    pub stream_header: xmltree::Element, //also contains desc
    pub stream_footer: xmltree::Element,

    pub samples: Vec<Sample>,
}

pub(crate) fn chunks_to_streams(chunks: Vec<Chunk>) -> Result<HashMap<u32, Stream>> {
    let mut file_header_chunk: Option<FileHeaderChunk> = None;
    let mut stream_header_chunks: Vec<StreamHeaderChunk> = Vec::new();
    let mut stream_footer_chunks: Vec<StreamFooterChunk> = Vec::new();
    let mut samples_chunks_map: HashMap<u32, Vec<SamplesChunk>> = HashMap::new();

    for chunk in chunks {
        match chunk {
            Chunk::FileHeaderChunk(c) => {
                file_header_chunk = Some(c);
            }
            Chunk::StreamHeaderChunk(c) => {
                stream_header_chunks.push(c);
            }
            Chunk::StreamFooterChunk(c) => {
                stream_footer_chunks.push(c);
            }
            Chunk::SamplesChunk(c) => {
                samples_chunks_map.entry(c.stream_id).or_insert(Vec::new()).push(c);
            }
            _ => {}
        }
    }

    let _ = file_header_chunk.ok_or(errors::ErrorKind::MissingFileHeaderChunk)?;

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


        // TODO should this work backwards too? Also think of edge cases.
        // deduce timestamps if not present but nominal_srate is specified.
        let mut most_recent_timestamp = None;
        let samples_vec = samples_chunks_map
            .entry(stream_id)
            .or_insert(Vec::new())
            .into_iter()
            .flat_map(|c| c.samples.clone())
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
        // let samples: &[Sample] = &samples_vec;

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
}
