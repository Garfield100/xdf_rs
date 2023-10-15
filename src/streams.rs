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


