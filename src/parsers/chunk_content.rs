// chunk structure
// [NumLengthBytes] [Length] [Tag] [Content]
// [1, 4, or 8] [...] [Tag number] [Arbitrary]
// [1] [As coded in NumLengthBytes] [2] [Variable]

use nom::{bytes::complete::take, error::context, IResult};
use tracing::{instrument, trace};

use super::chunk_length::length;

#[instrument(level = "trace", skip(input), ret)]
pub(crate) fn chunk_content(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, chunk_size) = context("chunk_content chunk_size", length)(input)?;
    trace!(%chunk_size);
    let (input, content) = context("chunk_content content", |i| take(chunk_size)(i))(input)?;

    Ok((input, content))
}
