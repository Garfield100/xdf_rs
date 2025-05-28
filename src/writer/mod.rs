// API design and builder pattern inspired by the CSV crate
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use stream_format::{NumberFormat, StreamFormat};

mod error;
mod stream_builder;
mod stream_format;
mod stream_writer;
mod timestamp;
mod xdf_builder;

use error::XDFWriterError;
use stream_builder::StreamBuilder;
pub use strict_num::NonZeroPositiveF64;
use timestamp::Timestamped;
pub use timestamp::{HasTimestamps, NoTimestamps};
pub use xdf_builder::XDFBuilder;
use xmltree::Element;

use crate::chunk_structs::{Tag};

pub type StreamID = u32;
const _: () = const {
    assert!(size_of::<StreamID>() == 4, "StreamID should be 4 bytes long");
};

pub struct XDFMeta {
    pub description: String,
    pub author: String,
    pub date: String,
}

pub(crate) struct WriteHelper<W: Write> {
    writer: W,
}

impl<W: Write> WriteHelper<W> {
    fn write_magic_num(&mut self) -> Result<(), std::io::Error> {
        const MAGIC_NUM: &[u8; 4] = b"XDF:";
        self.writer.write_all(MAGIC_NUM)?;
        Ok(())
    }

    pub(crate) fn write_file_header(&mut self, xml: &Element) -> Result<(), XDFWriterError> {
        self.write_magic_num()?;

        let mut xml_bytes = Vec::new();
        xml.write(&mut xml_bytes)?;

        self.write_chunk(Tag::FileHeader, &xml_bytes)?;

        Ok(())
    }

    pub(crate) fn write_stream_header(&mut self, id: StreamID, xml: &Element) -> Result<(), XDFWriterError> {
        let id_bytes = id.to_le_bytes();
        debug_assert!(id_bytes.len() == 4, "Stream ID should be 4 bytes long");

        let mut bytes = Vec::from(id_bytes);

        xml.write(&mut bytes)?;

        self.write_chunk(Tag::StreamHeader, &bytes)?;

        Ok(())
    }

    // in place to prematurely optimise away an allocation we already do in write_chunk
    pub(crate) fn length_helper(length: usize, dest: &mut Vec<u8>) {
        const U8_MAX: usize = u8::MAX as usize;
        const U8_MAX_1: usize = U8_MAX + 1;

        const U32_MAX: usize = u32::MAX as usize;
        const U32_MAX_1: usize = U32_MAX + 1;

        let num_length_bytes: u8 = match length {
            0..=U8_MAX => 1,
            U8_MAX_1..=U32_MAX => 4,
            U32_MAX_1.. => 8,
        };

        let length_bytes = &length.to_le_bytes()[..num_length_bytes as usize];

        dest.push(num_length_bytes);
        dest.extend_from_slice(length_bytes);
    }

    pub(crate) fn write_num_samples_chunk<F: StreamFormat + NumberFormat>(
        &mut self,
        id: StreamID,
        samples: &[F],
    ) -> Result<(), std::io::Error> {
        todo!()
    }

    pub(crate) fn write_str_samples_chunk(&mut self, id: StreamID, sample: &str) -> Result<(), std::io::Error> {
        todo!()
    }

    pub(crate) fn write_chunk(&mut self, chunk_tag: Tag, chunk_bytes: &[u8]) -> Result<(), std::io::Error> {
        // 2 tag bytes, 1 num length byte, max. 8 length bytes, and chunk bytes
        let mut bytes = Vec::with_capacity(2 + 1 + 8 + chunk_bytes.len());

        // two tag bytes which specify what kind of chunk it is
        let tag_bytes: [u8; 2] = chunk_tag.into();
        bytes.extend_from_slice(&tag_bytes);

        // one byte specifying the number of length bytes, and then N bytes containing the actual length
        Self::length_helper(chunk_bytes.len(), &mut bytes);

        // the chunk's actual byte content
        bytes.extend_from_slice(chunk_bytes);

        self.writer.write_all(&bytes)?;

        Ok(())
    }
}

pub(crate) struct SharedState<W: Write> {
    write_helper: WriteHelper<W>,
}

#[must_use]
pub struct XDFWriter<W: Write> {
    state: Arc<Mutex<SharedState<W>>>,
    num_streams: u32,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    channel_count: usize,
    nominal_srate: Option<NonZeroPositiveF64>,
    // name: String,
    // content_type: String,
}

impl StreamInfo {
    #[allow(clippy::must_use_candidate)] // false positive
    pub fn new(channel_count: usize, nominal_srate: Option<NonZeroPositiveF64>) -> Self {
        Self {
            channel_count,
            nominal_srate,
        }
    }
}

impl<W: Write> XDFWriter<W> {
    // only to be called by the XDFBuilder
    pub(crate) fn new(write_helper: WriteHelper<W>) -> Self {
        // the specification suggests ordinal numbers starting at 1
        Self {
            state: Arc::new(Mutex::new(SharedState { write_helper })),
            num_streams: 0,
        }
    }

    pub fn add_stream<F: StreamFormat, T: Timestamped>(&mut self, stream_info: StreamInfo) -> StreamBuilder<W, F, T> {
        // // Spec says to start at 1, so get the length after incrementing
        self.num_streams += 1;
        let stream_id = self.num_streams;

        StreamBuilder::new(stream_id, stream_info, self.state.clone())
    }
}
