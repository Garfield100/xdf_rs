use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use nom::{branch::alt, bytes::complete::tag, combinator::map, error::context, multi::many0, IResult};

use crate::chunk_structs::{Chunk, StreamHeaderChunkInfo};

use super::{boundary, clock_offset, file_header, samples, stream_footer, stream_header};

// structure of an XDF file:
// [MagicCode] [Chunk] [Chunk] [Chunk] ...
// [XDF:] [...] [...] [...] ...
// [4] [Variable] [Variable] [Variable] ...

//structure of a chunk:
// [NumLengthBytes] [Length] [Tag] [Content]
// [1, 4, or 8] [...] [Tag number] [Arbitrary]
// [1] [As coded in NumLengthBytes] [2] [Variable]

fn magic_number(input: &[u8]) -> IResult<&[u8], &[u8]> {
    context("magic_number", tag(b"XDF:"))(input)
}

// parses the magic number, the file header, and then all the rest of the chunks. Returns a vector of chunks
pub(crate) fn xdf_file_parser(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
    let stream_info_map: HashMap<u32, StreamHeaderChunkInfo> = HashMap::new();
    let cursed: Rc<RefCell<HashMap<u32, StreamHeaderChunkInfo>>> = Rc::new(RefCell::new(stream_info_map));

    let file_header_parser = map(file_header, Chunk::FileHeader);
    let mut file_header_parser = context("xdf_file file_header", file_header_parser);

    let stream_header_parser = map(stream_header, |stream_header_chunk| {
        let mut stream_info_map = cursed.deref().borrow_mut();
        stream_info_map.insert(stream_header_chunk.stream_id, stream_header_chunk.info.clone());
        Chunk::StreamHeader(stream_header_chunk)
    });
    let stream_header_parser = context("xdf_file stream_header", stream_header_parser);

    let samples_parser = map(|input| samples(input, cursed.clone()), Chunk::Samples);
    let samples_parser = context("xdf_file samples", samples_parser);

    let clock_offset_parser = map(clock_offset, Chunk::ClockOffset);
    let clock_offset_parser = context("xdf_file clock_offset", clock_offset_parser);

    let boundary_parser = map(boundary, Chunk::Boundary);
    let boundary_parser = context("xdf_file boundary", boundary_parser);

    let stream_footer_parser = map(stream_footer, Chunk::StreamFooter);
    let stream_footer_parser = context("xdf_file stream_footer", stream_footer_parser);

    let repeated_parsers = many0(alt((
        stream_header_parser,
        samples_parser,
        clock_offset_parser,
        boundary_parser,
        stream_footer_parser,
    )));
    let mut repeated_parsers = context("xdf_file repeated_parsers", repeated_parsers);

    let (input, _) = magic_number(input)?;
    let (input, file_header) = file_header_parser(input)?;
    let (input, other_chunks) = repeated_parsers(input)?;

    let mut chunks = vec![file_header];
    chunks.extend(other_chunks);

    Ok((input, chunks))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_xdf_file() {
        // load minimal.xdf which is included in the repo
        let input = include_bytes!("../../tests/read/minimal.xdf");

        let (rest, chunks) = xdf_file_parser(input).unwrap();

        assert_eq!(rest, &[] as &[u8]);
        assert_eq!(chunks.len(), 15);

        assert!(matches!(chunks[0], Chunk::FileHeader(_)));
        assert!(matches!(chunks[1], Chunk::StreamHeader(_)));
        assert!(matches!(chunks[2], Chunk::StreamHeader(_)));
        assert!(matches!(chunks[3], Chunk::Boundary(_)));
        assert!(matches!(chunks[4], Chunk::Samples(_)));
        assert!(matches!(chunks[5], Chunk::Samples(_)));
        assert!(matches!(chunks[6], Chunk::Samples(_)));
        assert!(matches!(chunks[7], Chunk::Samples(_)));
        assert!(matches!(chunks[8], Chunk::Samples(_)));
        assert!(matches!(chunks[9], Chunk::Samples(_)));
        assert!(matches!(chunks[10], Chunk::Boundary(_)));
        assert!(matches!(chunks[11], Chunk::ClockOffset(_)));
        assert!(matches!(chunks[12], Chunk::ClockOffset(_)));
        assert!(matches!(chunks[13], Chunk::StreamFooter(_)));
        assert!(matches!(chunks[14], Chunk::StreamFooter(_)));
    }
}
