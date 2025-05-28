use std::marker::PhantomData;

use xmltree::Element;

use crate::Format;

use super::{xdf_builder::xml_add_child_unchecked, StreamID, StreamInfo, XDFWriterError};
macro_rules! define_stream_type {
    ($name:ty, $format:expr) => {
        impl StreamFormat for $name {
            fn get_format() -> Format {
                $format
            }
        }
    };
}

pub(crate) trait StreamFormat: Sized {
    fn get_format() -> Format;
}

define_stream_type!(i8, Format::Int8);
define_stream_type!(i16, Format::Int16);
define_stream_type!(i32, Format::Int32);
define_stream_type!(i64, Format::Int64);
define_stream_type!(f32, Format::Float32);
define_stream_type!(f64, Format::Float64);
define_stream_type!(&str, Format::String);

pub(crate) trait NumberFormat {}
impl NumberFormat for i8 {}
impl NumberFormat for i16 {}
impl NumberFormat for i32 {}
impl NumberFormat for i64 {}
impl NumberFormat for f32 {}
impl NumberFormat for f64 {}

#[derive(Debug, Clone)]
pub struct StreamHandle<T: StreamFormat> {
    _format_marker: PhantomData<T>,
    // _writer_lifetime_marker: PhantomData<&'writer ()>,
    pub(crate) stream_info: StreamInfo,
    pub(crate) stream_id: StreamID,
}

impl<T: StreamFormat> StreamHandle<T> {
    // to be called by XDFWriter
    pub(crate) fn new(stream_id: StreamID, stream_info: StreamInfo) -> Self {
        Self {
            stream_id,
            stream_info,
            _format_marker: PhantomData,
            // _writer_lifetime_marker: PhantomData,
        }
    }

    pub(crate) fn chunk_bytes(&self) -> Result<Vec<u8>, XDFWriterError> {
        let id_bytes = self.stream_id.to_le_bytes();
        debug_assert!(id_bytes.len() == 4, "Stream ID should be 4 bytes");

        let mut bytes = id_bytes.to_vec();

        stream_xml_header::<T>(&self.stream_info).write(&mut bytes)?;

        Ok(bytes)
    }
}

fn stream_xml_header<T: StreamFormat>(stream_info: &StreamInfo) -> Element {
    let mut header = Element::new("info");
    xml_add_child_unchecked(&mut header, "channel_count", stream_info.channel_count.to_string());

    match stream_info.nominal_srate {
        Some(srate) => xml_add_child_unchecked(&mut header, "nominal_srate", srate.to_string()),
        None => xml_add_child_unchecked(&mut header, "nominal_srate", "0"),
    }

    xml_add_child_unchecked(&mut header, "format", String::from(T::get_format()));

    header
}
