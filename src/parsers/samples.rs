use std::{cell::RefCell, collections::HashMap, rc::Rc};

use nom::{
    combinator,
    error::context,
    multi,
    number::complete::{le_f64, u8},
    IResult,
};
use tracing::{instrument, trace};

use crate::{
    chunk_structs::{SamplesChunk, StreamHeaderChunkInfo},
    Format, Sample,
};

use super::{chunk_content, chunk_length::length, chunk_tags::samples_tag, stream_id, values};

#[instrument(level = "trace", skip(input), ret)]
fn optional_timestamp(input: &[u8]) -> IResult<&[u8], Option<f64>> {
    let (input, timestamp_bytes) = u8(input)?;
    trace!(%timestamp_bytes);

    match timestamp_bytes {
        0 => Ok((input, None)),
        8 => {
            let (input, timestamp) = le_f64(input)?;
            Ok((input, Some(timestamp)))
        }
        _ => Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Char, // not how these errors should be used but nom is a bit of a pain here
        ))),
    }
}

// structure of a sample:
// [TimeStampBytes] [OptionalTimeStamp] [Value 1] [Value 2] ... [Value N]
// [0 or 8] [Double, in seconds] [Value as defined by format] ...
// [1][8 if TimeStampBytes==8, 0 if TimeStampBytes==0] [[Variable]] ...
#[instrument(level = "trace", skip(input))]
fn sample(input: &[u8], num_channels: usize, format: Format) -> IResult<&[u8], Sample> {
    let (input, timestamp) = context("sample optional_timestamp", optional_timestamp)(input)?;
    let (input, values) = context("sample values", |i| values(i, format, num_channels))(input)?;

    Ok((input, Sample { timestamp, values }))
}

#[allow(clippy::needless_pass_by_value)]
#[instrument(level = "trace", skip(input))]
pub(super) fn samples(
    input: &[u8],
    stream_info: Rc<RefCell<HashMap<u32, StreamHeaderChunkInfo>>>,
) -> IResult<&[u8], SamplesChunk> {
    let stream_info = stream_info.borrow();
    let (input, chunk_content) = context("samples chunk_content", chunk_content)(input)?;
    let (chunk_content, _tag) = context("samples tag", samples_tag)(chunk_content)?; // 2 bytes
    let (chunk_content, stream_id) = context("samples stream_id", stream_id)(chunk_content)?; // 4 bytes
    let (chunk_content, num_samples) = context("samples num_samples", length)(chunk_content)?;

    let Some(stream_info) = stream_info.get(&stream_id) else {
        return context("samples get(&stream_id), missing a stream header", combinator::fail)(&[0]);
    };
    let num_channels = stream_info.channel_count as usize;
    let format = stream_info.channel_format;

    let (_chunk_content, samples) = multi::count(|i| sample(i, num_channels, format), num_samples)(chunk_content)?;

    Ok((input, SamplesChunk { stream_id, samples }))
}
