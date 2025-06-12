// API design and builder pattern inspired by the CSV crate
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use stream_format::StreamFormat;

mod error;
mod stream_builder;
pub mod stream_format;
mod stream_writer;
mod timestamp;
mod xdf_builder;

use error::XDFWriterError;
use stream_builder::StreamBuilder;
pub use strict_num::NonZeroPositiveF64;
use strict_num::PositiveF64;
use timestamp::TimestampTrait;
pub use timestamp::{HasTimestamps, NoTimestamps};
pub use xdf_builder::{HasMetadataAndDesc, XDFBuilder};
use xmltree::Element;

use crate::chunk_structs::Tag;

pub(crate) type StreamID = u32;

const _: () = const {
    assert!(size_of::<StreamID>() == 4, "StreamID should be 4 bytes long");
};

pub(crate) struct WriteHelper<W: Write> {
    writer: W,
}

fn length_helper(length: usize, num_length_bytes: u8) -> [u8; 9] {
    let mut arr = [0; 9];
    arr[0] = num_length_bytes;
    let length_bytes = length.to_le_bytes();
    arr[1..].copy_from_slice(&length_bytes);

    arr
}

macro_rules! length_bytes {
    ($length:expr) => {{
        use crate::writer::length_helper;
        const U8_MAX: usize = u8::MAX as usize;
        const U8_MAX_1: usize = U8_MAX + 1;

        const U32_MAX: usize = u32::MAX as usize;
        const U32_MAX_1: usize = U32_MAX + 1;

        let num_length_bytes: u8 = match $length {
            0..=U8_MAX => 1,
            U8_MAX_1..=U32_MAX => 4,
            U32_MAX_1.. => 8,
        };

        &length_helper($length, num_length_bytes)[..num_length_bytes as usize + 1]
    }};
}

pub(crate) use length_bytes;

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

        let mut bytes = Vec::from(id_bytes);

        xml.write(&mut bytes)?;

        self.write_chunk(Tag::StreamHeader, &bytes)?;

        Ok(())
    }

    pub(crate) fn write_stream_footer(&mut self, id: StreamID, xml: &Element) -> Result<(), XDFWriterError> {
        let id_bytes = id.to_le_bytes();
        let mut bytes = Vec::from(id_bytes);

        xml.write(&mut bytes)?;

        self.write_chunk(Tag::StreamFooter, &bytes)?;

        Ok(())
    }

    pub(crate) fn write_boundary(&mut self) -> Result<(), XDFWriterError> {
        const BOUNDARY_BYTES: [u8; 16] = [
            0x43, 0xA5, 0x46, 0xDC, 0xCB, 0xF5, 0x41, 0x0F, 0xB3, 0x0E, 0xD5, 0x46, 0x73, 0x83, 0xCB, 0xE4,
        ];

        self.write_chunk(Tag::Boundary, &BOUNDARY_BYTES)?;

        Ok(())
    }

    pub(crate) fn write_clock_offset(
        &mut self,
        id: StreamID,
        collection_time: PositiveF64,
        offset_value: PositiveF64,
    ) -> Result<(), XDFWriterError> {
        let id_bytes = id.to_le_bytes();
        let ct_bytes = collection_time.get().to_le_bytes();
        let off_bytes = offset_value.get().to_le_bytes();

        // one day we will be able to do this in place without unsafe qwq. Is optimised out anyway.
        let mut bytes: [u8; 20] = [0; 20];
        bytes[0..4].copy_from_slice(&id_bytes);
        bytes[4..12].copy_from_slice(&ct_bytes);
        bytes[12..20].copy_from_slice(&off_bytes);

        self.write_chunk(Tag::ClockOffset, &bytes)?;

        Ok(())
    }
    pub(crate) fn get_writer(&mut self) -> &mut W {
        &mut self.writer
    }

    fn write_chunk(&mut self, chunk_tag: Tag, chunk_bytes: &[u8]) -> Result<(), std::io::Error> {
        // 1 num length byte, max. 8 length bytes, 2 Tag bytes, and chunk bytes
        // let mut bytes = Vec::with_capacity(1 + 8 + 2 + chunk_bytes.len());

        // two tag bytes which specify what kind of chunk it is
        let tag_bytes: [u8; 2] = chunk_tag.as_bytes();

        // one byte specifying the number of length bytes, and then N bytes containing the actual length, including the tag
        self.writer
            .write_all(length_bytes!(chunk_bytes.len() + tag_bytes.len()))?;

        // bytes.extend_from_slice(&tag_bytes);
        self.writer.write_all(&tag_bytes)?;

        // the chunk's actual byte content
        // bytes.extend_from_slice(chunk_bytes);
        self.writer.write_all(chunk_bytes)?;

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
    pub channel_count: usize,
    pub nominal_srate: Option<NonZeroPositiveF64>,
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

    pub fn add_stream<F: StreamFormat, T: TimestampTrait>(
        &mut self,
        stream_info: StreamInfo,
    ) -> StreamBuilder<W, F, T> {
        // // Spec says to start at 1, so get the length after incrementing
        self.num_streams += 1;
        let stream_id = self.num_streams;

        StreamBuilder::new(stream_id, stream_info, self.state.clone())
    }

    pub fn write_boundary(&mut self) -> Result<(), XDFWriterError> {
        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;
        write_helper.write_boundary()
    }
}
