// clock offset structure
// [StreamID] [CollectionTime] [OffsetValue]
// [Ordinal number] [Double in seconds] [Double in seconds]
// [4] [8] [8]

use nom::{error::context, number::complete::le_f64, IResult};

use crate::ClockOffsetChunk;

use super::{chunk_content, chunk_tags::clock_offset_tag, stream_id};

pub(super) fn clock_offset(input: &[u8]) -> IResult<&[u8], ClockOffsetChunk> {
    let (input, chunk_content) = context("clock_offset chunk_content", chunk_content)(input)?;

    let (chunk_content, _tag) = context("clock_offset tag", clock_offset_tag)(chunk_content)?; // 2 bytes
    let (chunk_content, stream_id) = context("clock_offset stream_id", stream_id)(chunk_content)?; // 4 bytes
    let (chunk_content, collection_time) = context("clock_offset collection_time", le_f64)(chunk_content)?; // 8 bytes
    let (_chunk_content, offset_value) = context("clock_offset offset_value", le_f64)(chunk_content)?; // 8 bytes

    Ok((
        input,
        ClockOffsetChunk {
            stream_id,
            collection_time,
            offset_value,
        },
    ))
}
