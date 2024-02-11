use nom::{self, combinator::value, bytes::complete::tag, IResult, Parser, branch::alt};

use crate::chunk_structs::Tag;

// FileHeader tag parser
pub(crate) fn file_header_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::FileHeader, tag([1, 0])).parse(input)
}

// StreamHeader tag parser
pub(crate) fn stream_header_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::StreamHeader, tag([2, 0])).parse(input)
}

// Samples tag parser
pub(crate) fn samples_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::Samples, tag([3, 0])).parse(input)
}

// ClockOffset tag parser
pub(crate) fn clock_offset_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::ClockOffset, tag([4, 0])).parse(input)
}

// Boundary tag parser
pub(crate) fn boundary_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::Boundary, tag([5, 0])).parse(input)
}

// StreamFooter tag parser
pub(crate) fn stream_footer_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::StreamFooter, tag([6, 0])).parse(input)
}

// chunk tag parser
pub(crate) fn chunk_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    alt((
        file_header_tag,
        stream_header_tag,
        samples_tag,
        clock_offset_tag,
        boundary_tag,
        stream_footer_tag,
    ))
    .parse(input)
}
