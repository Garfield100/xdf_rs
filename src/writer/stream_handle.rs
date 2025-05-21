use std::marker::PhantomData;

use xmltree::Element;

use crate::Format;

use super::{StreamInfo, XDFWriterError};
macro_rules! define_stream_type {
    ($name:ident, $format:expr) => {
        pub struct $name;

        impl StreamFormat for $name {
            fn get_format() -> Format {
                $format
            }
        }
    };
}

pub(crate) trait StreamFormat {
    fn get_format() -> Format;
}

define_stream_type!(Int8Stream, Format::Int8);
define_stream_type!(Int16Stream, Format::Int16);
define_stream_type!(Int32Stream, Format::Int32);
define_stream_type!(Int64Stream, Format::Int64);
define_stream_type!(Float32Stream, Format::Float32);
define_stream_type!(Float64Stream, Format::Float64);
define_stream_type!(StringStream, Format::String);

#[derive(Debug)]
pub struct StreamHandle<'writer, T: StreamFormat> {
    _format_marker: PhantomData<T>,
    _writer_lifetime_marker: PhantomData<&'writer ()>,
    stream_info: StreamInfo,
    stream_id: u32,
}

impl<T: StreamFormat> StreamHandle<'_, T> {
    // to be called by XDFWriter
    pub(crate) fn new(stream_id: u32, stream_info: StreamInfo) -> Self {
        Self {
            stream_id,
            stream_info,
            _format_marker: PhantomData,
            _writer_lifetime_marker: PhantomData,
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
    header
        .attributes
        .insert("channel_count".to_string(), stream_info.channel_count.to_string());

    match stream_info.nominal_srate {
        Some(srate) => header.attributes.insert("nominal_srate".to_string(), srate.to_string()),
        None => header.attributes.insert("nominal_srate".to_string(), "0".to_string()),
    };

    header
        .attributes
        .insert("format".to_string(), String::from(T::get_format()));

    header
}
