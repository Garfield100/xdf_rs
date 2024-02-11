use xmltree::Element;

use crate::{Format, Sample};

#[derive(Debug)]
pub(crate) enum Chunk {
    FileHeader(FileHeaderChunk),
    StreamHeader(StreamHeaderChunk),
    Samples(SamplesChunk),
    ClockOffset(ClockOffsetChunk),
    Boundary(BoundaryChunk),
    StreamFooter(StreamFooterChunk),
}

#[derive(Debug)]
#[doc = "The FileHeaderChunk is the first chunk in an XDF file. It contains the version of the XDF file format and an XML element that contains additional information about the file."]
#[doc = "There must be exactly one FileHeaderChunk in an XDF file."]
pub(crate) struct FileHeaderChunk {
    /// The version of the XDF file format. Currently, only version 1.0 is supported.
    pub version: f32,
    /// The root of an XML element that contains additional information about the file.
    pub xml: Element,
}

// minimal tags in version 1.x:
// channel count
// nominal srate
// channel format

// common additional tags:
// name
// type
// desc

#[derive(Debug, Clone)]
pub(crate) struct StreamHeaderChunkInfo {
    pub channel_count: u32,
    pub nominal_srate: Option<f64>,
    pub channel_format: Format,

    pub name: Option<String>,
    pub r#type: Option<String>, // "type" is obviously a reserved keyword but can be escaped using r#
}

#[derive(Debug)]
pub(crate) struct StreamHeaderChunk {
    pub stream_id: u32,
    pub info: StreamHeaderChunkInfo,
    pub xml: Element,
}

#[derive(Debug)]
pub(crate) struct SamplesChunk {
    pub stream_id: u32,
    pub samples: Vec<Sample>,
}

//collection_time and offset_value are in seconds
#[derive(Debug)]
pub(crate) struct ClockOffsetChunk {
    pub stream_id: u32,
    pub collection_time: f64,
    pub offset_value: f64,
}

#[derive(Debug)]
pub(crate) struct BoundaryChunk {}

#[derive(Debug)]
pub(crate) struct StreamFooterChunk {
    pub stream_id: u32,
    pub xml: Element,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Tag {
    FileHeader,
    StreamHeader,
    Samples,
    ClockOffset,
    Boundary,
    StreamFooter,
}
