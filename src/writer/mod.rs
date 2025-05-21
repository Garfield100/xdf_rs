// API design and builder pattern inspired by the CSV crate
use std::io::{self, Write};

use stream_handle::{StreamFormat, StreamHandle};

mod stream_handle;
mod xdf_builder;

#[derive(thiserror::Error, Debug)]
pub enum XDFWriterError {
    #[error(transparent)]
    XMLTree(#[from] xmltree::Error),
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Conversion(#[from] std::num::TryFromIntError),
}

pub struct XDFMeta {
    pub description: String,
    pub author: String,
    pub date: String,
}

struct FooterInfo {
    first_timestamp: Option<f64>,
    last_timestamp: Option<f64>,
    sample_count: usize,
}

pub struct XDFWriter<Dest: Write> {
    writer: Dest,
    footer_info: Vec<FooterInfo>,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    channel_count: usize,
    nominal_srate: Option<f64>,
    name: String,
    content_type: String,
}

impl<Dest: Write> XDFWriter<Dest> {
    pub(crate) fn new(writer: Dest) -> Self {
        // the specification suggests ordinal numbers starting at 1
        Self {
            writer,
            footer_info: Vec::new(),
        }
    }

    pub fn add_stream<T: StreamFormat>(&mut self, stream_info: StreamInfo) -> Result<StreamHandle<T>, XDFWriterError> {
        self.footer_info.push(FooterInfo {
            first_timestamp: None,
            last_timestamp: None,
            sample_count: 0,
        });

        let stream_id = u32::try_from(self.footer_info.len())?;

        let handle = StreamHandle::new(stream_id, stream_info);
        self.writer.write_all(&handle.chunk_bytes()?)?;

        Ok(handle)
    }

    pub fn write_to_stream<T: StreamFormat>(&mut self, handle: &StreamHandle<T>) -> Result<(), XDFWriterError> {
        todo!()
    }
}
