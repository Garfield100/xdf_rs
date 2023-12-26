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

#[test]
fn file_header_chunk() {
    use crate::raw_chunks::read_to_raw_chunks;

    let start_bytes: Vec<u8> = vec![b'X', b'D', b'F', b':', 1, 0x3A, 1, 0];
    let xml_string = r#"<?xml version="1.0"?><info><version>1.0</version></info>"#;
    let bytes: Vec<u8> = [start_bytes, xml_string.as_bytes().to_vec()].concat();

    let res = read_to_raw_chunks(bytes.as_slice());

    let chunks = res.unwrap();

    assert_eq!(chunks.len(), 1);
    assert!(matches!(chunks[0].tag, Tag::FileHeader));
    assert_eq!(chunks[0].content_bytes.len(), 56);
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

#[derive(Debug)]
pub(crate) enum Tag {
    FileHeader,
    StreamHeader,
    Samples,
    ClockOffset,
    Boundary,
    StreamFooter,
}

// TODO: ensure correct visibility
#[derive(Debug)]
pub(crate) struct RawChunk {
    pub tag: Tag,
    pub content_bytes: Vec<u8>,
}
