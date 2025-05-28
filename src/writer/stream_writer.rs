// let stream = StreamWriter {b
//             state: self.state.clone(),
//             info: stream_info,
//             id: stream_id,
//             _timestamp_marker: PhantomData::<T>,
//             _format_marker: PhantomData::<F>,
//         };

//         let writer = &mut self.state.lock()?.writer;

use std::{
    io::Write,
    mem::size_of,
    sync::{Arc, Mutex},
};

use strict_num::PositiveF64;
use zerocopy::IntoBytes;

use crate::{chunk_structs::Tag, writer::length_bytes};

use super::{
    error::XDFWriterError,
    stream_format::{NumberFormat, StreamFormat},
    timestamp::Timestamped,
    SharedState, StreamID, StreamInfo,
};

pub struct StreamWriter<W: Write, F: StreamFormat, T: Timestamped> {
    pub(crate) state: Arc<Mutex<SharedState<W>>>,
    pub(crate) info: StreamInfo,
    pub(crate) id: StreamID,
    pub(crate) _timestamp_marker: std::marker::PhantomData<T>,
    pub(crate) _format_marker: std::marker::PhantomData<F>,
}

// impl<W: Write, F: StreamFormat, T: Timestamped> StreamWriter<W, F, T> {
//     pub fn end_stream(self) {
//         drop(self);
//     }
// }

impl<W: Write, F: StreamFormat, T: Timestamped> Drop for StreamWriter<W, F, T> {
    fn drop(&mut self) {
        todo!();
    }
}

// implementation for numeric types
impl<W: Write, F: StreamFormat + NumberFormat, T: Timestamped> StreamWriter<W, F, T> {
    pub fn write_samples(&mut self, samples: &[&[F]], first_timestamp: PositiveF64) -> Result<(), XDFWriterError> {
        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;

        // this is a bit ugly because to avoid allocating, we write the bytes
        // directly instead of going through the WriteHelper
        let writer = write_helper.get_writer();

        // size of one sample in bytes
        // size_of::<F>() * number of channels + 1 (timestamp indicator) + 0/8 (possibly timestamp)

        let sample_size = size_of::<F>() * self.info.channel_count + 1; // +1 for timestamp indicator
        let all_samples_size = sample_size * samples.len() + if T::is_timestamped() { 8 } else { 0 }; //only timestamp the first sample
        let samples_length_bytes = length_bytes!(all_samples_size);
        let stream_id_bytes: [u8; 4] = self.id.to_le_bytes();
        let samples_subchunk_size = size_of::<StreamID>() + samples_length_bytes.len() + all_samples_size;

        {
            // write the raw chunk header
            let chunk_length_bytes = length_bytes!(samples_subchunk_size);
            let tag_bytes: [u8; 2] = Tag::Samples.as_bytes();

            writer.write_all(chunk_length_bytes)?;
            writer.write_all(&tag_bytes)?;
        }

        writer.write_all(&stream_id_bytes)?;
        writer.write_all(samples_length_bytes)?;

        let mut first = true;

        // timestamp for the first one if applicable

        for &sample in samples {
            if sample.len() != self.info.channel_count {
                return Err(XDFWriterError::LengthMismatch {
                    expected: self.info.channel_count,
                    actual: sample.len(),
                });
            }
            if T::is_timestamped() && first {
                let mut bytes = [0_u8; 9];
                bytes[0] = 8; // indicate 8 bytes for the timestamp
                let first_timestamp_bytes = first_timestamp.get().to_le_bytes();
                bytes[1..].copy_from_slice(&first_timestamp_bytes);

                writer.write_all(&bytes)?;
            } else {
                // write 0 for the timestamp indicator
                writer.write_all(&[0])?;
            }
            first = false;

            // write values themselves
            let value_bytes: &[u8] = sample.as_bytes();
            writer.write_all(value_bytes)?;
        }

        todo!()
    }
}
