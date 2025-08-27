use nom::{error::context, IResult};

use crate::{
    chunk_structs::{StreamHeaderChunk, StreamHeaderChunkInfo},
    util::get_text_from_child,
    Format,
};

use super::{chunk_content, chunk_tags::stream_header_tag, stream_id, xml};

fn str_to_format(input: &str) -> Option<Format> {
    match input {
        "int8" => Some(Format::Int8),
        "int16" => Some(Format::Int16),
        "int32" => Some(Format::Int32),
        "int64" => Some(Format::Int64),
        "float32" => Some(Format::Float32),
        "double64" => Some(Format::Float64),
        "string" => Some(Format::String),
        _ => None,
    }
}

// StreamHeaderChunk contains streamID, info, and xml
// the info contains channel count, nominal_srate, format, name, and type
pub(crate) fn stream_header(input: &[u8]) -> IResult<&[u8], StreamHeaderChunk> {
    let (input, chunk_content) = context("stream_header chunk_content", chunk_content)(input)?;

    let (chunk_content, _) = context("stream_header tag", stream_header_tag)(chunk_content)?;
    let (chunk_content, stream_id) = context("stream_header stream_id", stream_id)(chunk_content)?;
    let (_chunk_content, xml) = context("stream_header xml", |i| xml(i))(chunk_content)?; // subtract 2 for the length field and 4 for the stream_id field

    let text_results = (
        get_text_from_child(&xml, "channel_count"),
        get_text_from_child(&xml, "nominal_srate"),
        get_text_from_child(&xml, "channel_format"),
    );

    let (Ok(channel_count_string), Ok(nominal_srate_string), Ok(format_string)) = text_results else {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Count, // not how these errors should be used but nom is a bit of a pain here
        )));
    };

    let (Some(channel_format), Some(channel_count)) =
        (str_to_format(&format_string), channel_count_string.parse::<u32>().ok())
    else {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Count, // not how these errors should be used but nom is a bit of a pain here
        )));
    };

    let nominal_srate = nominal_srate_string.parse::<f64>().ok();

    let name = get_text_from_child(&xml, "name").ok();
    let stream_type = get_text_from_child(&xml, "type").ok();

    let info = StreamHeaderChunkInfo {
        channel_count,
        nominal_srate,
        channel_format,
        name,
        stream_type,
    };

    Ok((input, StreamHeaderChunk { stream_id, info, xml }))
}
