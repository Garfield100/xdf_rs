use crate::{Format, Sample, StreamFormat, StreamID};

// minimal tags in version 1.0:
// channel count
// nominal srate
// channel format

// common additional tags:
// name
// type
// desc

// TODO add derives
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub id: u32, // not really necessary but nice for debugging and testing
    pub channel_count: u32,
    // TODO Use NonZeroPositiveF64 here
    pub nominal_srate: Option<f64>, //a mandatory field but we replace zero with None

    // optional fields:
    pub name: Option<String>,
    pub content_type: Option<String>,

    pub header: xmltree::Element, //contains desc
    pub footer: Option<xmltree::Element>,

    // TODO Use NonZeroPositiveF64 here
    pub measured_srate: Option<f64>,

    pub sample_enum: SampleEnum,
}

impl Stream {
    pub const fn format(&self) -> Format {
        self.sample_enum.format()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SampleEnum {
    Int8(Vec<Sample<i8>>),
    Int16(Vec<Sample<i16>>),
    Int32(Vec<Sample<i32>>),
    Int64(Vec<Sample<i64>>),
    Float32(Vec<Sample<f32>>),
    Float64(Vec<Sample<f64>>),
    String(Vec<Sample<String>>),
}

impl SampleEnum {
    pub const fn len(&self) -> usize {
        match self {
            SampleEnum::Int8(samples) => samples.len(),
            SampleEnum::Int16(samples) => samples.len(),
            SampleEnum::Int32(samples) => samples.len(),
            SampleEnum::Int64(samples) => samples.len(),
            SampleEnum::Float32(samples) => samples.len(),
            SampleEnum::Float64(samples) => samples.len(),
            SampleEnum::String(samples) => samples.len(),
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn format(&self) -> Format {
        match self {
            SampleEnum::Int8(_) => Format::Int8,
            SampleEnum::Int16(_) => Format::Int16,
            SampleEnum::Int32(_) => Format::Int32,
            SampleEnum::Int64(_) => Format::Int64,
            SampleEnum::Float32(_) => Format::Float32,
            SampleEnum::Float64(_) => Format::Float64,
            SampleEnum::String(_) => Format::String,
        }
    }
}
