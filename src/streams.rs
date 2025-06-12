use crate::{Format, Sample};

// minimal tags in version 1.0:
// channel count
// nominal srate
// channel format

// common additional tags:
// name
// type
// desc

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub id: u32, // not really necessary but nie for debugging and testing
    pub channel_count: u32,
    pub nominal_srate: Option<f64>, //a mandatory field but we replace zero with None
    pub format: Format,

    // optional fields:
    pub name: Option<String>,
    pub content_type: Option<String>,

    pub header: xmltree::Element, //contains desc
    pub footer: Option<xmltree::Element>,

    pub measured_srate: Option<f64>,

    pub samples: Vec<Sample>,
}
