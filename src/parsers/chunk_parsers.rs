use std::{
    borrow::{Borrow, BorrowMut, Cow},
    cell::RefCell,
    collections::HashMap,
    ops::Deref,
    rc::Rc,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::{fail, map},
    error::{context, convert_error, ParseError, VerboseError},
    multi::{self, many0},
    number::{
        self,
        complete::{le_f64, le_u32, u8},
    },
    sequence::tuple,
    Err, IResult, Parser,
};
use xmltree::Element;

use crate::{
    chunk_structs::{
        BoundaryChunk, Chunk, ClockOffsetChunk, FileHeaderChunk, SamplesChunk, StreamFooterChunk, StreamHeaderChunk,
        StreamHeaderChunkInfo,
    },
    util::{get_text_from_child, parse_version},
    Format, Sample, Values,
};

use super::{chunk_length::length, chunk_tags::*};

// structure of an XDF file:
// [MagicCode] [Chunk] [Chunk] [Chunk] ...
// [XDF:] [...] [...] [...] ...
// [4] [Variable] [Variable] [Variable] ...

//structure of a chunk:
// [NumLengthBytes] [Length] [Tag] [Content]
// [1, 4, or 8] [...] [Tag number] [Arbitrary]
// [1] [As coded in NumLengthBytes] [2] [Variable]

// structure of a value:
// [double, float, int64, int32, int16 or int8]
// [Arbitrary]
// [8, 4, 2 or 1]

// structure of a sample:
// [TimeStampBytes] [OptionalTimeStamp] [Value 1] [Value 2] ... [Value N]
// [0 or 8] [Double, in seconds] [Value as defined by format] ...
// [1][8 if TimeStampBytes==8, 0 if TimeStampBytes==0] [[Variable]] ...

//tags:
// 1: FileHeader (one per file)
// 2: StreamHeader (one per stream)
// 3: Samples (zero or more per stream)
// 4: ClockOffset (zero or more per stream)
// 5: Boundary (zero or more per file)
// 6: StreamFooter (one per stream)

pub(crate) fn xml(input: &[u8], length: usize) -> IResult<&[u8], Element> {
    let (input, content) = take(length).parse(input)?;
    let xml = Element::parse(content).unwrap();

    Ok((input, xml))
}

pub(crate) fn file_header(input: &[u8]) -> IResult<&[u8], FileHeaderChunk> {
    let (input, chunk_length) = context("file_header length", length)(input)?;
    let (input, _tag) = context("file_header tag", file_header_tag)(input)?;
    let (input, xml) = context("file_header xml", |i| xml(i, chunk_length - 2))(input)?;

    let version = parse_version(&xml)
        .map_err(|e| nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Float)))?;

    Ok((input, FileHeaderChunk { version, xml }))
}

fn stream_id(input: &[u8]) -> IResult<&[u8], u32> {
    le_u32(input)
}

fn str_to_format(input: &str) -> Option<Format> {
    match input {
        "int8" => Some(Format::Int8),
        "int16" => Some(Format::Int16),
        "int32" => Some(Format::Int32),
        "float32" => Some(Format::Float32),
        "double64" => Some(Format::Float64),
        "string" => Some(Format::String),
        _ => None,
    }
}

// StreamHeaderChunk contains streamID, info, and xml
// the info contains channel count, nominal_srate, format, name, and type
pub(crate) fn stream_header(input: &[u8]) -> IResult<&[u8], StreamHeaderChunk> {
    let (input, chunk_length) = context("stream_header length", length)(input)?;
    let (input, _) = context("stream_header tag", stream_header_tag)(input)?;
    let (input, stream_id) = context("stream_header stream_id", stream_id)(input)?;
    let (input, xml) = context("stream_header xml", |i| xml(i, chunk_length - 2 - 4))(input)?;

    let text_results = (
        get_text_from_child(&xml, "channel_count"),
        get_text_from_child(&xml, "nominal_srate"),
        get_text_from_child(&xml, "channel_format"),
    );

    let (channel_count_string, nominal_srate_strin, format_string) =
        if let (Ok(channel_count), Ok(nominal_srate), Ok(format)) = text_results {
            (channel_count, nominal_srate, format)
        } else {
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Count, //bad error kind but nom is a pain here
            )));
        };

    let (channel_format, channel_count) = if let (Some(format), Some(channel_count)) =
        (str_to_format(&format_string), channel_count_string.parse::<u32>().ok())
    {
        (format, channel_count)
    } else {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Count, //bad error kind but nom is a pain here
        )));
    };

    let nominal_srate = nominal_srate_strin.parse::<f64>().ok();

    let name = get_text_from_child(&xml, "name").ok();
    let r#type = get_text_from_child(&xml, "type").ok();

    let info = StreamHeaderChunkInfo {
        channel_count,
        nominal_srate,
        channel_format,
        name: name.map(String::from),
        r#type: r#type.map(String::from),
    };

    Ok((input, StreamHeaderChunk { stream_id, info, xml }))
}

// string value structure
// [NumLengthBytes] [Length] [StringContent]
// [1, 4, or 8] [...] [Arbitrary]
// [1] [As encoded] [Length]
fn string_value(input: &[u8]) -> IResult<&[u8], String> {
    let (input, length) = length(input)?;
    let (input, string) = nom::bytes::complete::take(length)(input)?;
    let string = String::from_utf8(string.to_vec()).unwrap();

    Ok((input, string))
}

fn values(input: &[u8], format: Format, num_values: usize) -> IResult<&[u8], Values> {
    let mut input = input;
    let values = match format {
        Format::Float32 => {
            let (inp, values) = context("values Float32", multi::count(number::complete::le_f32, num_values))(input)?;
            input = inp;
            Values::Float32(values)
        }
        Format::Float64 => {
            let (inp, values) = context("values Float64", multi::count(number::complete::le_f64, num_values))(input)?;
            input = inp;
            Values::Float64(values)
        }
        Format::Int8 => {
            let (inp, values) = context("values Int8", multi::count(number::complete::le_i8, num_values))(input)?;
            input = inp;
            Values::Int8(values)
        }
        Format::Int16 => {
            let (inp, values) = context("values Int16", multi::count(number::complete::le_i16, num_values))(input)?;
            input = inp;
            Values::Int16(values)
        }
        Format::Int32 => {
            let (inp, values) = context("values Int32", multi::count(number::complete::le_i32, num_values))(input)?;
            input = inp;
            Values::Int32(values)
        }
        Format::Int64 => {
            let (inp, values) = context("values Int64", multi::count(number::complete::le_i64, num_values))(input)?;
            input = inp;
            Values::Int64(values)
        }
        Format::String => {
            let (inp, string) = context("values String", string_value)(input)?;
            input = inp;
            Values::String(string)
        }
    };

    Ok((input, values))
}

fn optional_timestamp(input: &[u8]) -> IResult<&[u8], Option<f64>> {
    let (input, timestamp_bytes) = u8(input)?;
    match timestamp_bytes {
        0 => Ok((input, None)),
        8 => {
            let (input, timestamp) = le_f64(input)?;
            Ok((input, Some(timestamp)))
        }
        _ => Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Char,
        ))),
    }
}

fn sample(input: &[u8], num_channels: usize, format: Format) -> IResult<&[u8], Sample> {
    let (input, timestamp) = context("sample optional_timestamp", optional_timestamp)(input)?;
    let (input, values) = context("sample values", |i| values(i, format, num_channels))(input)?;

    Ok((input, Sample { timestamp, values }))
}

pub(crate) fn samples<'a>(
    input: &'a [u8],
    // stream_info: &HashMap<u32, StreamHeaderChunkInfo>,
    stream_info: Rc<RefCell<HashMap<u32, StreamHeaderChunkInfo>>>,
) -> IResult<&'a [u8], SamplesChunk> {
    let stream_info = stream_info.deref().borrow();

    let (input, _chunk_size) = context("samples chunk_size", length)(input)?;
    let (input, _tag) = context("samples tag", samples_tag)(input)?; // 2 bytes
    let (input, stream_id) = context("samples stream_id", stream_id)(input)?; // 4 bytes
    let (input, num_samples) = context("samples num_samples", length)(input)?;

    let stream_info = stream_info.get(&stream_id).unwrap();
    let num_channels = stream_info.channel_count as usize;
    let format = stream_info.channel_format;

    let (input, samples) = multi::count(|input| sample(input, num_channels, format), num_samples)(input)?;

    Ok((input, SamplesChunk { stream_id, samples }))
}

// clock offset structure
// [StreamID] [CollectionTime] [OffsetValue]
// [Ordinal number] [Double in seconds] [Double in seconds]
// [4] [8] [8]

pub(crate) fn clock_offset(input: &[u8]) -> IResult<&[u8], ClockOffsetChunk> {
    let (input, _chunk_size) = context("clock_offset chunk_size", length)(input)?;
    let (input, _tag) = context("clock_offset tag", clock_offset_tag)(input)?; // 2 bytes
    let (input, stream_id) = context("clock_offset stream_id", stream_id)(input)?; // 4 bytes
    let (input, collection_time) = context("clock_offset collection_time", le_f64)(input)?; // 8 bytes
    let (input, offset_value) = context("clock_offset offset_value", le_f64)(input)?; // 8 bytes

    Ok((
        input,
        ClockOffsetChunk {
            stream_id,
            collection_time,
            offset_value,
        },
    ))
}

// boundary structure
// [UUID]
// [0x43 0xA5 0x46 0xDC 0xCB 0xF5 0x41 0x0F 0xB3 0x0E 0xD5 0x46 0x73 0x83 0xCB 0xE4]
// [16]

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

fn magic_number(input: &[u8]) -> IResult<&[u8], &[u8]> {
    context("magic_number", tag(b"XDF:"))(input)
}

// xdf file parser
fn xdf_file(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
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
        let input = include_bytes!("../../tests/minimal.xdf");

        let (rest, chunks) = xdf_file(input).unwrap();

        assert_eq!(rest, &[] as &[u8]);
        assert_eq!(chunks.len(), 15);

        for chunk in chunks.iter() {
            println!("{:?}\n\n", chunk);
        }

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
