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
use tracing::trace;
use xmltree::Element;
use zerocopy::IntoBytes;

use crate::{
    chunk_structs::Tag,
    writer::{length_bytes, HasTimestamps, NoTimestamps},
};

use super::{
    error::XDFWriterError,
    stream_format::{NumberFormat, StreamFormat},
    timestamp::TimestampTrait,
    xdf_builder::xml_add_child_unchecked,
    SharedState, StreamID, StreamInfo,
};

pub struct StreamWriter<W: Write, F: StreamFormat, T: TimestampTrait> {
    pub(crate) state: Arc<Mutex<SharedState<W>>>,
    pub(crate) info: StreamInfo,
    pub(crate) id: StreamID,
    pub(crate) first_timestamp: Option<PositiveF64>,
    pub(crate) last_timestamp: Option<PositiveF64>,
    pub(crate) num_samples_written: usize,
    pub(crate) _timestamp_marker: std::marker::PhantomData<T>,
    pub(crate) _format_marker: std::marker::PhantomData<F>,
}

// impl<W: Write, F: StreamFormat, T: Timestamped> StreamWriter<W, F, T> {
//     pub fn end_stream(self) {
//         drop(self);
//     }
// }

impl<W: Write, F: StreamFormat, T: TimestampTrait> Drop for StreamWriter<W, F, T> {
    fn drop(&mut self) {
        self.close_helper().expect("Failed to close stream writer properly");
    }
}

// general functions independent of stream type
impl<W: Write, F: StreamFormat, T: TimestampTrait> StreamWriter<W, F, T> {
    pub fn close(mut self) -> Result<(), XDFWriterError> {
        // write the stream footer
        self.close_helper()?;

        Ok(())
    }

    // TODO tests
    pub fn write_boundary(&mut self) -> Result<(), XDFWriterError> {
        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;
        write_helper.write_boundary()
    }

    fn close_helper(&mut self) -> Result<(), XDFWriterError> {
        // write the stream footer
        let mut footer_xml = Element::new("info");
        if let Some(first_timestamp) = self.first_timestamp {
            xml_add_child_unchecked(&mut footer_xml, "first_timestamp", first_timestamp.get().to_string());
        }
        if let Some(last_timestamp) = self.last_timestamp {
            xml_add_child_unchecked(&mut footer_xml, "last_timestamp", last_timestamp.get().to_string());
        }
        xml_add_child_unchecked(
            &mut footer_xml,
            "num_samples_written",
            self.num_samples_written.to_string(),
        );

        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;

        write_helper.write_stream_footer(self.id, &footer_xml)?;

        Ok(())
    }
}

// TODO minimise critical section
// implementation for timestamped number types
impl<W, F> StreamWriter<W, F, HasTimestamps>
where
    W: Write,
    F: StreamFormat + NumberFormat,
{
    pub fn write_samples<S: AsRef<[F]>>(
        &mut self,
        samples: &[S],
        first_timestamp: PositiveF64,
    ) -> Result<(), XDFWriterError> {
        if samples.is_empty() {
            return Ok(()); // nothing to write, avoid locking
        }

        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;

        // this is a bit ugly because to avoid allocating, we write the bytes
        // directly instead of going through the WriteHelper
        let writer = write_helper.get_writer();

        // size of one sample in bytes
        // size_of::<F>() * number of channels + 1 (timestamp indicator) + 0/8 (possibly timestamp)

        let sample_size = size_of::<F>() * self.info.channel_count + 1; // +1 for timestamp indicator
        let all_samples_size = sample_size * samples.len() + 8; //only timestamp the first sample
                                                                // let samples_length_bytes = length_bytes!(all_samples_size);
        let samples_length_bytes = length_bytes!(samples.len());
        let stream_id_bytes: [u8; 4] = self.id.to_le_bytes();
        let samples_subchunk_size = size_of::<StreamID>() + samples_length_bytes.len() + all_samples_size;

        {
            // write the raw chunk header
            let tag_bytes: [u8; 2] = Tag::Samples.as_bytes();
            let chunk_length_bytes = length_bytes!(samples_subchunk_size + tag_bytes.len());

            writer.write_all(chunk_length_bytes)?;
            writer.write_all(&tag_bytes)?;
        }

        writer.write_all(&stream_id_bytes)?;
        writer.write_all(samples_length_bytes)?;

        let mut first = true;

        // timestamp for the first one if applicable

        for sample in samples {
            if sample.as_ref().len() != self.info.channel_count {
                return Err(XDFWriterError::LengthMismatch {
                    expected: self.info.channel_count,
                    actual: sample.as_ref().len(),
                });
            }
            if first {
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
            let value_bytes: &[u8] = (*sample.as_ref()).as_bytes();
            writer.write_all(value_bytes)?;
        }

        // assumption: this function is called in order of timestamps
        // update information for the footer
        self.num_samples_written += samples.len();
        self.first_timestamp.get_or_insert(first_timestamp);

        // update the last timestamp
        // if we have an srate, we can calculate the timestamp of the last sample given to us
        if let Some(srate) = self.info.nominal_srate {
            // TODO period could be calculated once on Stream creation and stored
            let period = 1.0 / srate.get(); // result is still non-zero (barring subnormals) and positive
            let time_delta = first_timestamp.get() + samples.len() as f64 * period; // same for this
            let last_timestamp = first_timestamp.get() + time_delta; // and this

            // this should therefore be safe to unwrap
            let last_timestamp = PositiveF64::new(last_timestamp).expect("last timestamp must be positive and finite");

            self.last_timestamp.replace(last_timestamp);
        } else {
            // if there is no srate, the best we can do is use the only timestamp given
            self.last_timestamp.replace(first_timestamp);
        }

        Ok(())
    }
}

// implementation for number types without timestamps
impl<W, F> StreamWriter<W, F, NoTimestamps>
where
    W: Write,
    F: StreamFormat + NumberFormat,
{
    pub fn write_samples<S: AsRef<[F]>>(&mut self, samples: &[S]) -> Result<(), XDFWriterError> {
        if samples.is_empty() {
            return Ok(()); // nothing to write, avoid locking
        }

        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;

        // this is a bit ugly because to avoid allocating, we write the bytes
        // directly instead of going through the WriteHelper
        let writer = write_helper.get_writer();

        // size of one sample in bytes
        let sample_size = size_of::<F>() * self.info.channel_count + 1; // +1 for timestamp indicator (always 0 here)
        let all_samples_size = sample_size * samples.len();
        let samples_length_bytes = length_bytes!(samples.len());
        let stream_id_bytes: [u8; 4] = self.id.to_le_bytes();
        let samples_subchunk_size = size_of::<StreamID>() + samples_length_bytes.len() + all_samples_size;

        {
            // write the raw chunk header
            let tag_bytes: [u8; 2] = Tag::Samples.as_bytes();
            let chunk_length_bytes = length_bytes!(samples_subchunk_size + tag_bytes.len());

            writer.write_all(chunk_length_bytes)?;
            writer.write_all(&tag_bytes)?;
        }

        writer.write_all(&stream_id_bytes)?;
        writer.write_all(samples_length_bytes)?;

        // timestamp for the first one if applicable

        for sample in samples {
            if sample.as_ref().len() != self.info.channel_count {
                return Err(XDFWriterError::LengthMismatch {
                    expected: self.info.channel_count,
                    actual: sample.as_ref().len(),
                });
            }
            writer.write_all(&[0])?;

            // write values themselves
            let value_bytes: &[u8] = (*sample.as_ref()).as_bytes();
            writer.write_all(value_bytes)?;
        }

        // update information for the footer
        self.num_samples_written += samples.len();

        Ok(())
    }
}

// implementation for string streams with timestamps
impl<W> StreamWriter<W, &str, HasTimestamps>
where
    W: Write,
{
    pub fn write_samples<'a, S: AsRef<[&'a str]>>(
        &mut self,
        samples: &[S],
        first_timestamp: PositiveF64,
    ) -> Result<(), XDFWriterError> {
        if samples.is_empty() {
            return Ok(()); // nothing to write, avoid locking
        }

        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;
        let writer = write_helper.get_writer();
        // let mut all_length_bytes = Vec::with_capacity(samples.len() * 2); // estimate most strings to be shorter than 255 bytes
        // let mut total_strings_size: usize = 0; //size of the strings in bytes, without any length bytes
        // let mut num_length_bytes: usize = 0; // total number of length bytes used for all strings
        // let mut sample_lengths: Vec<usize> = Vec::with_capacity(samples.len());
        let mut sample_lengths_sum: usize = 0; // the sum of all samples in bytes, including their timestamps and length bytes

        // because we can't know the size of the strings for this chunk in advance, we have to go through it twice

        // first pass: calculate each sample's size
        {
            let mut is_first_sample = true;
            for sample in samples {
                let sample = sample.as_ref();
                if sample.len() != self.info.channel_count {
                    return Err(XDFWriterError::LengthMismatch {
                        expected: self.info.channel_count,
                        actual: sample.len(),
                    });
                }

                let mut sample_content_byte_size: usize = 1; // 1 byte for the timestamp indicator

                if is_first_sample {
                    sample_content_byte_size += 8; // 8 bytes for the timestamp in the first sample
                    is_first_sample = false;
                }

                for &value in sample {
                    trace!("String Value: {value}");

                    let value_bytes_len = value.len(); //str::len returns byte length

                    // TODO investigate whether it's better to allocate and store these length bytes or to just recalculate later
                    let value_length_bytes_len = length_bytes!(value_bytes_len).len();
                    sample_content_byte_size += value_bytes_len + value_length_bytes_len;

                    // num_length_bytes += value_length_bytes.len();
                    // total_strings_size += value_bytes.len();
                    // all_length_bytes.push(value_length_bytes);
                }

                let sample_length_bytes = length_bytes!(sample_content_byte_size);

                trace!("Writing string sample of length {sample_content_byte_size} and length bytes: {sample_length_bytes:?}.");

                let sample_byte_length = sample_content_byte_size + sample_length_bytes.len();
                sample_lengths_sum += sample_byte_length;
            }
        }
        // second pass, now we can write

        let samples_length_bytes = length_bytes!(samples.len());
        let stream_id_bytes: [u8; 4] = self.id.to_le_bytes();
        let samples_subchunk_size = stream_id_bytes.len() + samples_length_bytes.len() + sample_lengths_sum;
        {
            // write the raw chunk header
            let tag_bytes: [u8; 2] = Tag::Samples.as_bytes();
            let chunk_length_bytes = length_bytes!(samples_subchunk_size + tag_bytes.len());

            writer.write_all(chunk_length_bytes)?;
            writer.write_all(&tag_bytes)?;
        }

        writer.write_all(&stream_id_bytes)?;
        writer.write_all(samples_length_bytes)?;

        // timestamp for the first one if applicable
        let mut first = true;
        for sample in samples {
            if first {
                // TODO just write directly instead of combining and then writing?
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
            for &str_value in sample.as_ref() {
                let str_content_bytes = str_value.as_bytes();
                let value_length_bytes = length_bytes!(str_content_bytes.len());

                writer.write_all(value_length_bytes)?;
                writer.write_all(str_content_bytes)?;
            }
        }

        // update the last timestamp
        // if we have an srate, we can calculate the timestamp of the last sample given to us
        if let Some(srate) = self.info.nominal_srate {
            let period = 1.0 / srate.get(); // result is still non-zero (barring subnormals) and positive
            let time_delta = first_timestamp.get() + samples.len() as f64 * period; // same for this
            let last_timestamp = first_timestamp.get() + time_delta; // and this

            // this should therefore be safe to unwrap
            let last_timestamp = PositiveF64::new(last_timestamp).expect("last timestamp must be positive and finite");

            self.last_timestamp.replace(last_timestamp);
        } else {
            // if there is no srate, the best we can do is use the only timestamp given
            self.last_timestamp.replace(first_timestamp);
        }

        self.num_samples_written += samples.len();

        Ok(())
    }
}
