// boundary structure
// [UUID]
// [0x43 0xA5 0x46 0xDC 0xCB 0xF5 0x41 0x0F 0xB3 0x0E 0xD5 0x46 0x73 0x83 0xCB 0xE4]
// [16]

use nom::{bytes::complete::tag, error::context, IResult};
use tracing::instrument;

use crate::BoundaryChunk;

use super::{chunk_content, chunk_tags::boundary_tag};

#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn boundary(input: &[u8]) -> IResult<&[u8], BoundaryChunk> {
    let (input, chunk_content) = context("boundary chunk_content", chunk_content)(input)?;

    let (chunk_content, _tag) = context("boundary tag", boundary_tag)(chunk_content)?; // 2 bytes
    let (_chunk_content, _boundary_bytes) = context(
        "boundary boundary_bytes",
        tag([
            0x43, 0xA5, 0x46, 0xDC, 0xCB, 0xF5, 0x41, 0x0F, 0xB3, 0x0E, 0xD5, 0x46, 0x73, 0x83, 0xCB, 0xE4,
        ]),
    )(chunk_content)?;

    Ok((input, BoundaryChunk {}))
}
