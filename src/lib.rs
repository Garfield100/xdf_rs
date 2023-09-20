#![forbid(unsafe_code)]
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
// use byteorder::{ByteOrder, LittleEndian};
// use errors::ParseChunkError;
// use thiserror::Error;
// use xmltree::Element;

use std::{collections::HashMap, io::Read};

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
        let header_xml: xmltree::Element = chunks
            .iter()
            .find_map(|c| match c {
                Chunk::FileHeaderChunk(c) => Some(c.xml.clone()),
                _ => None,
            })
            .ok_or(errors::ErrorKind::MissingFileHeaderChunk)?;

        let streams = streams::chunks_to_streams(chunks)?;

        Ok(Self {
            header: header_xml,
            streams,
        })
    }
}
