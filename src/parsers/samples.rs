use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use nom::{
    combinator,
    error::context,
    multi,
    number::complete::{le_f64, u8},
    IResult,
};

use crate::{Format, Sample, SamplesChunk, StreamHeaderChunkInfo};

use super::{chunk_length::length, chunk_tags::samples_tag, stream_id, values};

fn optional_timestamp(input: &[u8]) -> IResult<&[u8], Option<f64>> {
    let (input, timestamp_bytes) = u8(input)?;
    match timestamp_bytes {
        0 => Ok((input, None)),
        8 => {
            let (input, timestamp) = le_f64(input)?;
            Ok((input, Some(timestamp)))
        }
        _ => Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Char,
        ))),
    }
}

// structure of a sample:
// [TimeStampBytes] [OptionalTimeStamp] [Value 1] [Value 2] ... [Value N]
// [0 or 8] [Double, in seconds] [Value as defined by format] ...
// [1][8 if TimeStampBytes==8, 0 if TimeStampBytes==0] [[Variable]] ...

fn sample(input: &[u8], num_channels: usize, format: Format) -> IResult<&[u8], Sample> {
    let (input, timestamp) = context("sample optional_timestamp", optional_timestamp)(input)?;
    let (input, values) = context("sample values", |i| values(i, format, num_channels))(input)?;

    Ok((input, Sample { timestamp, values }))
}

pub(super) fn samples(
    input: &[u8],
    // stream_info: &HashMap<u32, StreamHeaderChunkInfo>,
    stream_info: Rc<RefCell<HashMap<u32, StreamHeaderChunkInfo>>>,
) -> IResult<&[u8], SamplesChunk> {
    let stream_info = stream_info.deref().borrow();

    let (input, _chunk_size) = context("samples chunk_size", length)(input)?;
    let (input, _tag) = context("samples tag", samples_tag)(input)?; // 2 bytes
    let (input, stream_id) = context("samples stream_id", stream_id)(input)?; // 4 bytes
    let (input, num_samples) = context("samples num_samples", length)(input)?;

    let stream_info = match stream_info.get(&stream_id) {
        Some(stream_info) => stream_info,
        // nom errors are a bit painful
        None => return context("samples get(&stream_id), missing a stream header", combinator::fail)(&[0]),
    };
    let num_channels = stream_info.channel_count as usize;
    let format = stream_info.channel_format;

    let (input, samples) = multi::count(|input| sample(input, num_channels, format), num_samples)(input)?;

    Ok((input, SamplesChunk { stream_id, samples }))
}
