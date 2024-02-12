use nom::{error::context, IResult};

use crate::StreamFooterChunk;

use super::{chunk_length::length, chunk_tags::stream_footer_tag, stream_id, xml};

// stream footer structure
// [StreamID] [XML UTF8 string]
// [Ordinal number] [[Valid XML]]
// [4] [As determined by chunk length]

pub(crate) fn stream_footer(input: &[u8]) -> IResult<&[u8], StreamFooterChunk> {
    let (input, chunk_size) = context("stream_footer chunk_size", length)(input)?;

    let (input, _) = context("stream_footer tag", stream_footer_tag)(input)?; // 2 bytes
    let (input, stream_id) = context("stream_footer stream_id", stream_id)(input)?; // 4 bytes
    let (input, xml) = context("stream_footer xml", |i| xml(i, chunk_size - 2 - 4))(input)?;

    Ok((input, StreamFooterChunk { stream_id, xml }))
}
