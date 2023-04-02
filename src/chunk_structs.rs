use xmltree::Element;

#[derive(Debug)]
pub struct FileHeaderChunk {
    pub version: f32,
    pub xml: Element,
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
#[derive(Debug)]
pub struct StreamHeaderChunkInfo {
    pub name: String,
    pub r#type: String, // "type" is obviously a reserved keyword but can be escaped using r#
    pub channel_count: u32,
    pub nominal_srate: f64,
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
pub struct StreamHeaderChunk {
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
pub struct SamplesChunk {
    pub stream_id: u32,
    pub samples: Vec<Sample>,
}

//collection_time and offset_value are in seconds
#[derive(Debug)]
pub struct ClockOffsetChunk {
    pub stream_id: u32,
    pub collection_time: f64,
    pub offset_value: f64,
}

#[derive(Debug)]
pub struct BoundaryChunk {}

//TODO: check what fields are and are not really mandatory
//If we don't have first or last timestamps given, is it ok if we instead
//determine those ourselves?
#[derive(Debug)]
pub struct StreamFooterChunkInfo {
    pub first_timestamp: Option<f64>,
    pub last_timestamp: Option<f64>,
    pub sample_count: u64,
    pub measured_srate: f64,
}

#[derive(Debug)]
pub struct StreamFooterChunk {
    pub stream_id: u32,
    pub info: StreamFooterChunkInfo,
    pub xml: Element,
}

#[derive(Debug)]
pub enum Tag {
    FileHeader,
    StreamHeader,
    Samples,
    ClockOffset,
    Boundary,
    StreamFooter,
}

// TODO: ensure correct visibility
#[derive(Debug)]
pub struct RawChunk {
    pub tag: Tag,
    pub content_bytes: Vec<u8>,
}
