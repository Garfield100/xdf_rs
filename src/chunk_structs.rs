use xmltree::Element;

#[derive(Debug)]
pub(crate) enum Chunk {
    FileHeaderChunk(FileHeaderChunk),
    StreamHeaderChunk(StreamHeaderChunk),
    SamplesChunk(SamplesChunk),
    ClockOffsetChunk(ClockOffsetChunk),
    BoundaryChunk(BoundaryChunk),
    StreamFooterChunk(StreamFooterChunk),
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum Format {
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
#[derive(Debug)]
pub enum Value {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
}

//TODO: check what fields are and are not really mandatory.
// so the minimal.xdf example file contains exactly the fields you can see in
// the struct below. Strangely, the xml example in the xdf specification
// includes more fields:
// https://github.com/sccn/xdf/wiki/Specifications#streamheader-chunk
// I have decided to go with the more minimal of the two so as not to error on
// the most minimal.
#[derive(Debug, Clone)]
pub(crate) struct StreamHeaderChunkInfo {
    pub name: String,
    pub r#type: String, // "type" is obviously a reserved keyword but can be escaped using r#
    pub channel_count: u32,
    pub nominal_srate: Option<f64>,
    pub channel_format: Format,
    //source_id
    //version
    pub created_at: f64,
    //uid
    //session_id
    //hostname
    pub desc: Option<Element>,
}

#[derive(Debug)]
pub(crate) struct StreamHeaderChunk {
    pub stream_id: u32,
    pub info: StreamHeaderChunkInfo,
    pub xml: Element,
}

#[derive(Debug)]
pub struct Sample {
    pub timestamp: Option<f64>,
    pub values: Vec<Value>,
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

//TODO: check what fields are and are not really mandatory
//If we don't have first or last timestamps given, is it ok if we instead
//determine those ourselves?
#[derive(Debug)]
pub(crate) struct StreamFooterChunkInfo {
    pub first_timestamp: Option<f64>,
    pub last_timestamp: Option<f64>,
    pub sample_count: u64,
    pub measured_srate: Option<f64>,
}

#[derive(Debug)]
pub(crate) struct StreamFooterChunk {
    pub stream_id: u32,
    pub info: StreamFooterChunkInfo,
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
