// clock offset structure
// [StreamID] [CollectionTime] [OffsetValue]
// [Ordinal number] [Double in seconds] [Double in seconds]
// [4] [8] [8]

use nom::{error::context, number::complete::le_f64, IResult};

use crate::ClockOffsetChunk;

use super::{chunk_length::length, chunk_tags::clock_offset_tag, stream_id};

pub(super) fn clock_offset(input: &[u8]) -> IResult<&[u8], ClockOffsetChunk> {
    let (input, _chunk_size) = context("clock_offset chunk_size", length)(input)?;
    let (input, _tag) = context("clock_offset tag", clock_offset_tag)(input)?; // 2 bytes
    let (input, stream_id) = context("clock_offset stream_id", stream_id)(input)?; // 4 bytes
    let (input, collection_time) = context("clock_offset collection_time", le_f64)(input)?; // 8 bytes
    let (input, offset_value) = context("clock_offset offset_value", le_f64)(input)?; // 8 bytes

    Ok((
        input,
        ClockOffsetChunk {
            stream_id,
            collection_time,
            offset_value,
        },
    ))
}
