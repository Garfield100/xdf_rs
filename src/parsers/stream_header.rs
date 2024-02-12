use nom::{error::context, IResult};

use crate::{util::get_text_from_child, Format, StreamHeaderChunk, StreamHeaderChunkInfo};

use super::{chunk_length::length, chunk_tags::stream_header_tag, stream_id, xml};

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
    let stream_type = get_text_from_child(&xml, "type").ok();

    let info = StreamHeaderChunkInfo {
        channel_count,
        nominal_srate,
        channel_format,
        name: name.map(String::from),
        stream_type: stream_type.map(String::from),
    };

    Ok((input, StreamHeaderChunk { stream_id, info, xml }))
}
