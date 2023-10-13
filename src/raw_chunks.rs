use byteorder::{ByteOrder, LittleEndian}; // TODO use T::from_le_bytes() instead
use xmltree::Element;

use core::slice;
use std::{collections::HashMap, io::Read};

use crate::{
    chunk_structs::*,
    errors::{self, ParseChunkError, ReadChunkError},
    util::{extract_timestamp, get_text_from_child, opt_string_to_f64, parse_version},
    Format, Sample, Value,
};

pub(crate) fn read_to_raw_chunks(file_bytes: &[u8]) -> errors::Result<Vec<RawChunk>> {
    let mut raw_chunks: Vec<RawChunk> = Vec::new();
    let mut file_header_found: bool = false;

    let mut content_iter = file_bytes.into_iter().map(|b| *b).enumerate();

    for _ in 0..4 {
        let (index, byte) = content_iter.next().ok_or(ReadChunkError::EOFError)?;
        if byte != "XDF:".as_bytes()[index] {
            return Err(ReadChunkError::NoMagicNumberError.into());
        }
    }

    while let Some(num_length_bytes) = content_iter.next() {
        let mut chunk_length: u64;
        match num_length_bytes.1 {
            1 => chunk_length = content_iter.next().unwrap().1 as u64,
            4 | 8 => {
                let mut bytes: Vec<u8> = vec![0; num_length_bytes.1 as usize];
                for i in 0..bytes.len() {
                    if let Some(next_byte) = content_iter.next() {
                        bytes[i] = next_byte.1;
                    } else {
                        return Err(ReadChunkError::EOFError.into());
                    }
                }

                chunk_length = match num_length_bytes.1 {
                    4 => LittleEndian::read_u32(&bytes) as u64,
                    8 => LittleEndian::read_u64(&bytes),
                    _ => unreachable!(),
                }
            }

            _ => {
                return Err(ReadChunkError::ParseError(format!(
                    "Invalid number of chunk length bytes found at index {}. Expected 1, 4, or 8 but was {}",
                    num_length_bytes.0, num_length_bytes.1
                ))
                .into());
            }
        }

        let mut tag_bytes: [u8; 2] = [0; 2];
        for i in 0..tag_bytes.len() {
            tag_bytes[i] = {
                let val = content_iter.next();
                match val {
                    Some(val) => val.1,
                    None => return Err(ReadChunkError::EOFError.into()),
                }
            }
            .clone();
        }

        let chunk_tag_num = LittleEndian::read_u16(&tag_bytes);

        let chunk_tag: Tag = match chunk_tag_num {
            1 => {
                if file_header_found {
                    return Err(ReadChunkError::ParseError(format!("More than one FileHeaders found.")).into());
                }
                file_header_found = true;
                Tag::FileHeader
            }
            2 => Tag::StreamHeader,
            3 => Tag::Samples,
            4 => Tag::ClockOffset,
            5 => Tag::Boundary,
            6 => Tag::StreamFooter,
            _ => return Err(ReadChunkError::InvalidTagError(chunk_tag_num).into()),
        };

        //subtract the two tag bytes for the content length
        chunk_length -= 2;

        // try to cast the chunk length to usize in order to allocate a vector with it
        let chunk_length: usize = match (chunk_length).try_into() {
            Ok(len) => len,
            Err(err) => {
                return Err(ReadChunkError::ParseError(format!(
                    "Chunk too big. Cannot cast {} to usize\n{}",
                    chunk_length, err
                ))
                .into());
            }
        };

        let mut chunk_bytes: Vec<u8> = vec![0; chunk_length];
        for i in 0..chunk_length {
            chunk_bytes[i] = {
                match content_iter.next() {
                    Some(val) => val.1,
                    None => return Err(ReadChunkError::EOFError.into()),
                }
            };
        }

        let raw_chunk = RawChunk {
            tag: chunk_tag,
            content_bytes: chunk_bytes,
        };

        raw_chunks.push(raw_chunk);
    }

    if !file_header_found {
        return Err(ReadChunkError::ParseError(format!("No FileHeader found.")).into());
    }

    return Ok(raw_chunks);
}

// yes these are ugly, they were extracted by refactoring
#[inline]
pub(crate) fn parse_stream_footer(
    raw_chunk: RawChunk,
    stream_num_samples_map: &HashMap<u32, u64>,
    stream_info_map: &HashMap<u32, StreamHeaderChunkInfo>,
) -> Result<Chunk, errors::Error> {
    let id_bytes = &raw_chunk.content_bytes[..4];
    let stream_id: u32 = LittleEndian::read_u32(id_bytes);
    let root = {
        match Element::parse(&raw_chunk.content_bytes[4..]) {
            Ok(root) => root,
            Err(err) => return Err(ParseChunkError::XMLParseError(err).into()),
        }
    };
    let first_timestamp_str = get_text_from_child(&root, "first_timestamp").ok();
    let last_timestamp_str = get_text_from_child(&root, "last_timestamp").ok();
    let measured_srate_str = get_text_from_child(&root, "measured_srate").ok();
    let first_timestamp = opt_string_to_f64(first_timestamp_str)?;
    let last_timestamp = opt_string_to_f64(last_timestamp_str)?;
    let stream_info = stream_info_map.get(&stream_id).unwrap();

    let measured_srate = if let Some(_) = stream_info.nominal_srate {
        Some(opt_string_to_f64(measured_srate_str)?.unwrap_or_else(|| {
            // measured_srate is missing, so we calculate it ourselves

            // nominal_srate is given as "a floating point number in Hertz. If the stream
            // has an irregular sampling rate (that is, the samples are not spaced evenly in
            // time, for example in an event stream), this value must be 0."

            if let (Some(num_samples), Some(first_timestamp), Some(last_timestamp)) =
                (stream_num_samples_map.get(&stream_id), first_timestamp, last_timestamp)
            {
                if *num_samples == 0 {
                    0.0 // don't divide by zero :)
                } else {
                    (last_timestamp - first_timestamp) / *num_samples as f64
                }
            } else {
                0.0
            }
        }))
    } else {
        None
    };
    let info = StreamFooterChunkInfo {
        first_timestamp,
        last_timestamp,
        sample_count: get_text_from_child(&root, "sample_count")?
            .parse()
            .map_err(|err| ParseChunkError::BadElementError(format!("Error while parsing sample count: {}", err)))?,
        measured_srate,
    };
    let stream_footer_chunk = Chunk::StreamFooterChunk(StreamFooterChunk {
        stream_id,
        info,
        xml: root,
    });
    Ok(stream_footer_chunk)
}

#[inline]
pub(crate) fn parse_samples(
    raw_chunk: RawChunk,
    stream_num_samples_map: &mut HashMap<u32, u64>,
    stream_id: u32,
    stream_info_map: &HashMap<u32, StreamHeaderChunkInfo>,
) -> Result<Chunk, errors::Error> {
    let num_samples_byte_num = &raw_chunk.content_bytes[4];
    match num_samples_byte_num {
        1 | 4 | 8 => (),
        _ => {
            return Err(ParseChunkError::InvalidChunkBytesError {
                msg: format!(
                    "Invalid amount of sample number bytes: was {} but expected 1, 4, or 8.",
                    num_samples_byte_num
                ),
                raw_chunk_bytes: raw_chunk.content_bytes,
                raw_chunk_tag: 3, //always 3 because we are in the match arm for the samples tag
                offset: 4,
            }
            .into());
        }
    }
    let num_samples_bytes = &raw_chunk.content_bytes[5..(5 + num_samples_byte_num) as usize];
    let num_samples: u64 = LittleEndian::read_uint(num_samples_bytes, *num_samples_byte_num as usize);
    if stream_num_samples_map.contains_key(&stream_id) {
        stream_num_samples_map.insert(stream_id, stream_num_samples_map.get(&stream_id).unwrap() + num_samples);
    } else {
        stream_num_samples_map.insert(stream_id, num_samples);
    }
    let stream_info = stream_info_map
        .get(&stream_id)
        .ok_or(ParseChunkError::MissingHeaderError { stream_id })?;
    let type_size: Option<i32> = match stream_info.channel_format {
        Format::Int8 => Some(1),
        Format::Int16 => Some(2),
        Format::Int32 => Some(4),
        Format::Int64 => Some(8),
        Format::Float32 => Some(4),
        Format::Float64 => Some(8),
        Format::String => None,
    };
    let mut offset: usize = 4 + 1 + *num_samples_byte_num as usize;
    let mut samples: Vec<Sample> = Vec::with_capacity(num_samples as usize);
    if let Some(type_size) = type_size {
        //constant size types
        for _ in 0..num_samples {
            // let mut values: Vec<Value> = Vec::with_capacity(stream_info.channel_count as usize);
            let timestamp: Option<f64> = extract_timestamp(&raw_chunk, &mut offset);

            // realign the whole slice directly
            let values_bytes =
                &raw_chunk.content_bytes[offset..offset + (type_size as usize * stream_info.channel_count as usize)];
            let values: Vec<Value> = match stream_info.channel_format {
                Format::Int8 => bytemuck::cast_slice::<u8, i8>(values_bytes)
                    .iter()
                    .map(|&v| Value::Int8(v))
                    .collect(),
                Format::Int16 => {
                    let mut vec_for_alignment: Vec<i16> = vec![0; values_bytes.len() / 2];

                    #[allow(unused_mut)] // marking the variables as mut because they are mutated in unsafe code
                    let mut vec_for_alignment_as_bytes =
                        bytemuck::cast_slice::<i16, u8>(vec_for_alignment.as_mut_slice());
                    let mutable_bytes;
                    unsafe {
                        mutable_bytes = slice::from_raw_parts_mut(
                            vec_for_alignment_as_bytes.as_ptr() as *mut _,
                            vec_for_alignment_as_bytes.len(),
                        );
                        mutable_bytes.copy_from_slice(values_bytes);
                    }

                    bytemuck::cast_slice::<u8, i16>(mutable_bytes)
                        .iter()
                        .map(|&v| Value::Int16(v))
                        .collect()
                }
                Format::Int32 => {
                    let mut vec_for_alignment: Vec<i32> = vec![0; values_bytes.len() / 4];

                    #[allow(unused_mut)] // marking the variables as mut because they are mutated in unsafe code
                    let mut vec_for_alignment_as_bytes =
                        bytemuck::cast_slice::<i32, u8>(vec_for_alignment.as_mut_slice());
                    let mutable_bytes;
                    unsafe {
                        mutable_bytes = slice::from_raw_parts_mut(
                            vec_for_alignment_as_bytes.as_ptr() as *mut _,
                            vec_for_alignment_as_bytes.len(),
                        );
                        mutable_bytes.copy_from_slice(values_bytes);
                    }

                    bytemuck::cast_slice::<u8, i32>(mutable_bytes)
                        .iter()
                        .map(|&v| Value::Int32(v))
                        .collect()
                }
                Format::Int64 => {
                    let mut vec_for_alignment: Vec<i64> = vec![0; values_bytes.len() / 8];

                    #[allow(unused_mut)] // marking the variables as mut because they are mutated in unsafe code
                    let mut vec_for_alignment_as_bytes =
                        bytemuck::cast_slice::<i64, u8>(vec_for_alignment.as_mut_slice());
                    let mutable_bytes;
                    unsafe {
                        mutable_bytes = slice::from_raw_parts_mut(
                            vec_for_alignment_as_bytes.as_ptr() as *mut _,
                            vec_for_alignment_as_bytes.len(),
                        );
                        mutable_bytes.copy_from_slice(values_bytes);
                    }

                    bytemuck::cast_slice::<u8, i64>(mutable_bytes)
                        .iter()
                        .map(|&v| Value::Int64(v))
                        .collect()
                }
                Format::Float32 => {
                    let mut vec_for_alignment: Vec<f32> = vec![0.0; values_bytes.len() / 4];

                    #[allow(unused_mut)] // marking the variables as mut because they are mutated in unsafe code
                    let mut vec_for_alignment_as_bytes =
                        bytemuck::cast_slice::<f32, u8>(vec_for_alignment.as_mut_slice());
                    let mutable_bytes;
                    unsafe {
                        mutable_bytes = slice::from_raw_parts_mut(
                            vec_for_alignment_as_bytes.as_ptr() as *mut _,
                            vec_for_alignment_as_bytes.len(),
                        );
                        mutable_bytes.copy_from_slice(values_bytes);
                    }

                    bytemuck::cast_slice::<u8, f32>(mutable_bytes)
                        .iter()
                        .map(|&v| Value::Float32(v))
                        .collect()
                }
                Format::Float64 => {
                    let mut vec_for_alignment: Vec<f64> = vec![0.0; values_bytes.len() / 8];

                    #[allow(unused_mut)] // marking the variables as mut because they are mutated in unsafe code
                    let mut vec_for_alignment_as_bytes =
                        bytemuck::cast_slice::<f64, u8>(vec_for_alignment.as_mut_slice());
                    let mutable_bytes;
                    unsafe {
                        mutable_bytes = slice::from_raw_parts_mut(
                            vec_for_alignment_as_bytes.as_ptr() as *mut _,
                            vec_for_alignment_as_bytes.len(),
                        );
                        mutable_bytes.copy_from_slice(values_bytes);
                    }

                    bytemuck::cast_slice::<u8, f64>(mutable_bytes)
                        .iter()
                        .map(|&v| Value::Float64(v))
                        .collect()
                }
                Format::String => unreachable!(),
            };
            offset += type_size as usize * stream_info.channel_count as usize;

            samples.push(Sample { timestamp, values });
        }
    } else {
        //strings
        for _ in 0..num_samples {
            let timestamp: Option<f64> = extract_timestamp(&raw_chunk, &mut offset);
            let num_length_bytes: usize = raw_chunk.content_bytes[offset] as usize;
            offset += 1; //for number of length bytes field
            let value_length = match num_length_bytes {
                1 => raw_chunk.content_bytes[offset] as u64,
                4 => u32::from_le_bytes((&raw_chunk.content_bytes[offset..(offset + num_length_bytes)]).try_into()?)
                    as u64,
                8 => u64::from_le_bytes((&raw_chunk.content_bytes[offset..(offset + num_length_bytes)]).try_into()?),
                _ => {
                    let msg = format!(
                        "Error: Number of length bytes for this value are invalid. Expected 1, 4 or 8 but got {}",
                        num_length_bytes
                    );
                    return Err(ParseChunkError::InvalidChunkBytesError {
                        msg,
                        raw_chunk_bytes: raw_chunk.content_bytes,
                        raw_chunk_tag: 3, // always 3 because we are in the match arm for the samples tag
                        offset,
                    }
                    .into());
                }
            } as usize;
            offset += num_length_bytes; // for length field
            let mut value_bytes = &raw_chunk.content_bytes[offset..(offset + value_length)];

            // Turn the bytes into a valid utf-8 string
            let mut value_string = String::new();
            if let Err(err) = value_bytes.read_to_string(&mut value_string) {
                return Err(ParseChunkError::Utf8Error(err).into());
            };

            let value = Value::String(value_string);
            let value_vec = vec![value];

            samples.push(Sample {
                timestamp,
                values: value_vec,
            });
            offset += value_length; // for value field
        }
    }
    let samples_chunk = Chunk::SamplesChunk(SamplesChunk { stream_id, samples });
    Ok(samples_chunk)
}

#[inline]
pub(crate) fn parse_stream_header(
    raw_chunk: &RawChunk,
    stream_info_map: &mut HashMap<u32, StreamHeaderChunkInfo>,
) -> Result<Chunk, errors::Error> {
    let id_bytes = &raw_chunk.content_bytes[..4];
    let stream_id: u32 = LittleEndian::read_u32(id_bytes);
    let root = {
        match Element::parse(&raw_chunk.content_bytes[4..]) {
            Ok(root) => root,
            Err(err) => return Err(ParseChunkError::XMLParseError(err).into()),
        }
    };
    let info = StreamHeaderChunkInfo {
        name: Some(get_text_from_child(&root, "name")?),
        r#type: Some(get_text_from_child(&root, "type")?),
        channel_count: get_text_from_child(&root, "channel_count")?
            .parse()
            .map_err(|err| ParseChunkError::BadElementError(format!("Error while parsing channel count: {}", err)))?,
        nominal_srate: Some(
            get_text_from_child(&root, "nominal_srate")?.parse().map_err(|err| {
                ParseChunkError::BadElementError(format!("Error while parsing channel count: {}", err))
            })?,
        ),
        channel_format: match get_text_from_child(&root, "channel_format")?.to_lowercase().as_str() {
            "in8" => Format::Int8,
            "int16" => Format::Int16,
            "int32" => Format::Int32,
            "int64" => Format::Int64,
            "float32" => Format::Float32,
            "double64" => Format::Float64,
            "string" => Format::String,
            invalid => {
                return Err(ParseChunkError::BadElementError(format!("Invalid stream format \"{}\"", invalid)).into())
            }
        },
        desc: match root.get_child("desc") {
            Some(desc) => Some(desc.clone()),
            None => None,
        },
    };
    stream_info_map.insert(stream_id, info.clone());
    let stream_header_chunk = StreamHeaderChunk {
        stream_id,
        info,
        xml: root,
    };
    Ok(Chunk::StreamHeaderChunk(stream_header_chunk))
}

// tests
#[test]
fn empty_file() {
    let bytes: Vec<u8> = vec![];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(
        res.unwrap_err(),
        errors::Error(errors::ErrorKind::ReadChunkError(ReadChunkError::EOFError), _)
    ));
}

#[test]
fn file_too_short() {
    let bytes: Vec<u8> = vec![b'X'];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(
        res.unwrap_err(),
        errors::Error(errors::ErrorKind::ReadChunkError(ReadChunkError::EOFError), _)
    ));
}

#[test]
fn no_magic_number() {
    let bytes: Vec<u8> = vec![b'X', b'D', b'A', b':'];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(matches!(
        res.unwrap_err(),
        errors::Error(errors::ErrorKind::ReadChunkError(ReadChunkError::NoMagicNumberError), _)
    ));
}

#[test]
fn invalid_tags() {
    //tag 0 is invalid
    let bytes: Vec<u8> = vec![b'X', b'D', b'F', b':', 1, 3, 0, 0, 10];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(
        matches!(res.unwrap_err(), errors::Error(errors::ErrorKind::ReadChunkError(ReadChunkError::InvalidTagError(invalid_tag)), _) if invalid_tag == 0)
    );

    //tag 7 is invalid
    let bytes: Vec<u8> = vec![b'X', b'D', b'F', b':', 1, 3, 7, 0, 10];
    let res = read_to_raw_chunks(bytes.as_slice());
    assert!(
        matches!(res.unwrap_err(), errors::Error(errors::ErrorKind::ReadChunkError(ReadChunkError::InvalidTagError(invalid_tag)), _) if invalid_tag == 7)
    );
}
