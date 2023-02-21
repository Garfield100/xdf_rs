use minidom::Element;

pub struct FileHeaderChunk {
    version: f32,
    xml: Element,
}

//TODO: check what fields are and are not really mandatory.
struct StreamHeaderChunkInfo<Format> {
    name: String,
    r#type: String, // "type" is obviously a reserved keyword but can be escaped using r#
    channel_count: u32,
    nominal_srate: f64,
    channel_format: Format,
    created_at: f64,
    desc: Element,
}

pub struct StreamHeaderChunk<Format> {
    stream_id: u32,
    info: StreamHeaderChunkInfo<Format>,
    xml: Element,
}

pub struct Sample<T> {
    timestamp: Option<f64>,
    values: Vec<T>,
}

pub struct SamplesChunk<'a, T> {
    stream_id: u32,
    samples: Vec<&'a Sample<T>>,
}

// TODO: check whether or not these offsets can be negative
pub struct ClockOffsetChunk {
    stream_id: u32,
    collection_time: u64,
    offset_value: u64,
}

pub struct BoundaryChunk {}

//TODO: check what fields are and are not really mandatory. If we don't have
//first or last timestamps given, is it ok if we instead determine those
//ourselves?
struct StreamFooterChunkInfo {
    first_timestamp: Option<f64>,
    last_timestamp: Option<f64>,
    sample_count: u64,
    measured_srate: f64,
}

pub struct StreamFooterChunk {
    stream_id: u32,
    info: StreamFooterChunkInfo,
    xml: Element,
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
    pub(crate) tag: Tag,
    pub(crate) content_bytes: Vec<u8>,
}