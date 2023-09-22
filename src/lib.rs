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

use std::collections::HashMap;

mod chunk_structs;
mod errors;
mod raw_chunks;
mod streams;
mod util;

use raw_chunks::*;
use streams::Stream;

// use crate::chunk_structs::*;
use crate::chunk_structs::Chunk;

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

//This is a little annoying. Do I remove the channel_format and Fomat struct
//above entirely and just use the type of the sample's vector elements?
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Value {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
}
#[derive(Debug, PartialEq, Clone)]
pub struct Sample {
    pub timestamp: Option<f64>,
    pub values: Vec<Value>,
}

impl XDFFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::errors::Error> {
        let raw_chunks = read_to_raw_chunks(bytes)?;
        let chunks = raw_chunks_to_chunks(raw_chunks)?;
        let file_header_xml: xmltree::Element = if let Some(Chunk::FileHeaderChunk(c)) = chunks.get(0) {
            c.xml.clone()
        } else {
            return Err(crate::errors::ErrorKind::MissingFileHeaderChunk.into());
        };

        let streams = streams::chunks_to_streams(chunks)?;

        Ok(Self {
            header: file_header_xml,
            streams,
        })
    }
}
