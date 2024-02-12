// boundary structure
// [UUID]
// [0x43 0xA5 0x46 0xDC 0xCB 0xF5 0x41 0x0F 0xB3 0x0E 0xD5 0x46 0x73 0x83 0xCB 0xE4]
// [16]

use nom::{bytes::complete::tag, error::context, IResult};

use crate::BoundaryChunk;

use super::{chunk_length::length, chunk_tags::boundary_tag};

pub(crate) fn boundary(input: &[u8]) -> IResult<&[u8], BoundaryChunk> {
    let (input, _chunk_size) = context("boundary chunk_size", length)(input)?;
    let (input, _tag) = context("boundary tag", boundary_tag)(input)?; // 2 bytes
    let (input, _boundary_bytes) = context(
        "boundary boundary_bytes",
        tag([
            0x43, 0xA5, 0x46, 0xDC, 0xCB, 0xF5, 0x41, 0x0F, 0xB3, 0x0E, 0xD5, 0x46, 0x73, 0x83, 0xCB, 0xE4,
        ]),
    )(input)?;

    Ok((input, BoundaryChunk {}))
}
