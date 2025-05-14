use nom::{error::context, IResult};

use crate::StreamFooterChunk;

use super::{chunk_content, chunk_tags::stream_footer_tag, stream_id, xml};

// stream footer structure
// [StreamID] [XML UTF8 string]
// [Ordinal number] [[Valid XML]]
// [4] [As determined by chunk length]

pub(crate) fn stream_footer(input: &[u8]) -> IResult<&[u8], StreamFooterChunk> {
    let (input, chunk_content) = context("stream_footer chunk_content", chunk_content)(input)?;

    let (chunk_content, _) = context("stream_footer tag", stream_footer_tag)(chunk_content)?; // 2 bytes
    let (chunk_content, stream_id) = context("stream_footer stream_id", stream_id)(chunk_content)?; // 4 bytes
    let (_chunk_content, xml) = context("stream_footer xml", |i| xml(i))(chunk_content)?;

    Ok((input, StreamFooterChunk { stream_id, xml }))
}
