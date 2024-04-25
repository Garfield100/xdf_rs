use nom::{error::context, IResult};

use crate::{util::parse_version, FileHeaderChunk};

use super::{chunk_length::length, chunk_tags::file_header_tag, xml::xml};

pub(crate) fn file_header(input: &[u8]) -> IResult<&[u8], FileHeaderChunk> {
    let (input, chunk_length) = context("file_header length", length)(input)?;
    let (input, _tag) = context("file_header tag", file_header_tag)(input)?;
    let (input, xml) = context("file_header xml", |i| xml(i, chunk_length - 2))(input)?;

    let version = parse_version(&xml)
        .map_err(|_e| nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Float)))?; // not how these errors should be used but nom is a bit of a pain here

    Ok((input, FileHeaderChunk { version, xml }))
}
