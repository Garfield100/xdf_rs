// API design and builder pattern inspired by the CSV crate
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use crate::stream_format;
use stream_format::StreamFormat;

mod error;
mod stream_builder;

/// Contains valid stream format types and traits.
mod stream_writer;
mod timestamp;
pub(crate) mod write_helper;
mod xdf_builder;

use error::XDFWriterError;
use stream_builder::StreamBuilder;
pub use strict_num::NonZeroPositiveF64;
use timestamp::TimestampTrait;
pub use timestamp::{HasTimestamps, NoTimestamps};
pub use xdf_builder::{HasMetadataAndDesc, XDFBuilder};

use crate::{writer::write_helper::WriteHelper, StreamID};

pub(crate) trait Sealed {}

const _: () = const {
    assert!(size_of::<StreamID>() == 4, "StreamID should be 4 bytes long");
};

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

#[derive(Debug)]
pub(crate) struct SharedState<W: Write> {
    write_helper: WriteHelper<W>,
}

/// Used to add new streams to the file.
///
/// XDFWriters are returned by finalising an XDFBuilder using its [`build`](XDFBuilder::build) method.
/// Their main function is to add new streams to the XDF File being written using the [`add_stream`](XDFWriter::add_stream) method.
///
/// See
/// TODO add link to XDFBuilder doc example
#[must_use]
#[derive(Debug, Clone)] // Can be safely cloned since it does not contain its own state
pub struct XDFWriter<W: Write> {
    state: Arc<Mutex<SharedState<W>>>,
    num_streams: u32,
}

/// Basic stream information used when constructing a new stream.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// This stream's channel count. All samples pushed to this stream must have this size.
    pub channel_count: usize,

    /// The ideal rate of this stream's samples in Hertz, if present.
    pub nominal_srate: Option<NonZeroPositiveF64>,
}

impl StreamInfo {
    /// Creates a new [`StreamInfo`] struct.
    #[expect(clippy::must_use_candidate)] // false positive
    pub const fn new(channel_count: usize, nominal_srate: Option<NonZeroPositiveF64>) -> Self {
        Self {
            channel_count,
            nominal_srate,
        }
    }
}

impl<W: Write> XDFWriter<W> {
    /// Only to be called by the XDFBuilder
    pub(crate) fn new(write_helper: WriteHelper<W>) -> Self {
        // the specification suggests ordinal numbers starting at 1
        Self {
            state: Arc::new(Mutex::new(SharedState { write_helper })),
            num_streams: 0,
        }
    }

    /// The way to introduce new streams to a file. Returns a new [`StreamBuilder`].
    ///
    /// In addition to a [`StreamInfo`] struct, this function takes two generic parameters:
    /// a type implementing [`StreamFormat`], and either [`HasTimestamps`] or [`NoTimestamps`].
    /// The first specifies which format the stream should have, i.e. the type of its values, for example i8 or f32.
    /// The second specifies whether or not this stream's samples should contain timestamps or not.
    ///
    /// # Examples
    ///
    /// ```
    ///
    ///
    /// ```
    // TODO finish example
    pub fn add_stream<F: StreamFormat, T: TimestampTrait>(
        &mut self,
        stream_info: StreamInfo,
    ) -> StreamBuilder<W, F, T> {
        // // Spec says to start at 1, so get the length after incrementing
        self.num_streams += 1;
        let stream_id = self.num_streams;

        StreamBuilder::new(stream_id, stream_info, self.state.clone())
    }

    /// Writes a boundary chunk to the file.
    ///
    /// Boundary chunks contain a 16 long string of bytes which can be used to recover some intact chunks if part of a file has been corrupted.
    /// The XDF Specification recommends writing one of these boundary chunks around every 10 seconds.
    /// For convenience, this same function can also be called from [`StreamWriter`]s.
    pub fn write_boundary(&mut self) -> Result<(), XDFWriterError> {
        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;
        write_helper.write_boundary()
    }
}
