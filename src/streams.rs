use std::collections::HashMap;

use crate::chunk_structs::{Chunk, FileHeaderChunk, Sample, SamplesChunk, StreamFooterChunk, StreamHeaderChunk};
use crate::errors::{self, Result};

#[derive(Debug)]
pub struct Stream {
    pub stream_id: u32,
    pub first_timestamp: Option<f64>,
    pub last_timestamp: Option<f64>,
    pub sample_count: u64,
    pub measured_srate: Option<f64>,
    pub stream_header: xmltree::Element,
    pub stream_footer: xmltree::Element,
    pub samples: Vec<Sample>,
}

pub(crate) fn chunks_to_streams(chunks: Vec<Chunk>) -> Result<Vec<Stream>> {
    let mut file_header_chunk: Option<FileHeaderChunk> = None;
    let mut stream_header_chunks: Vec<StreamHeaderChunk> = Vec::new();
    let mut stream_footer_chunks: Vec<StreamFooterChunk> = Vec::new();
    let mut samples_chunks: Vec<SamplesChunk> = Vec::new();

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
                samples_chunks.push(c);
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

        let stream = Stream {
            stream_id,
            first_timestamp: stream_footer.info.first_timestamp,
            last_timestamp: stream_footer.info.last_timestamp,
            sample_count: stream_footer.info.sample_count,
            measured_srate: stream_footer.info.measured_srate,
            stream_header: stream_header.xml.clone(),
            stream_footer: stream_footer.xml.clone(),
            samples: Vec::new(),
        };

        streams_map.insert(stream_id, stream);
    }

    for mut samples_chunk in samples_chunks {
        let samples_stream_id = samples_chunk.stream_id;
        let stream = streams_map
            .get_mut(&samples_stream_id)
            .ok_or(errors::ErrorKind::MissingStreamHeaderChunk(samples_stream_id))?;

        stream.samples.append(&mut samples_chunk.samples);
    }

    Ok(streams_map.into_values().collect())
}
