use nom::{self, bytes::complete::tag, combinator::value, IResult, Parser};
use tracing::instrument;

use crate::chunk_structs::Tag;

//tags:
// 1: FileHeader (one per file)
// 2: StreamHeader (one per stream)
// 3: Samples (zero or more per stream)
// 4: ClockOffset (zero or more per stream)
// 5: Boundary (zero or more per file)
// 6: StreamFooter (one per stream)

// FileHeader tag parser
#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn file_header_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::FileHeader, tag([1, 0])).parse(input)
}

// StreamHeader tag parser
#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn stream_header_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::StreamHeader, tag([2, 0])).parse(input)
}

// Samples tag parser
#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn samples_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::Samples, tag([3, 0])).parse(input)
}

// ClockOffset tag parser
#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn clock_offset_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::ClockOffset, tag([4, 0])).parse(input)
}

// Boundary tag parser
#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn boundary_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::Boundary, tag([5, 0])).parse(input)
}

// StreamFooter tag parser
#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn stream_footer_tag(input: &[u8]) -> IResult<&[u8], Tag> {
    value(Tag::StreamFooter, tag([6, 0])).parse(input)
}
