
use std::rc::Rc;



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
    pub channel_count: u32,
    pub nominal_srate: Option<f64>, //a mandatory field but we replace zero with None
    pub format: Format,

    // optional fields:
    pub name: Option<Rc<str>>,
    pub r#type: Option<Rc<str>>,

    pub stream_header: xmltree::Element, //contains desc
    pub stream_footer: Option<xmltree::Element>,

    pub measured_srate: Option<f64>,

    pub samples: Vec<Sample>,
}
