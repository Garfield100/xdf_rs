use byteorder::{ByteOrder, LittleEndian};
use xmltree::Element;

use std::{
    collections::HashMap,
    io::{BufReader, Read},
};

use crate::{
    chunk_structs::*,
    errors::{self, ParseChunkError, ReadChunkError},
    util::{extract_timestamp, get_text_from_child, opt_string_to_f64, parse_version},
    Format, Sample, Value,
};

pub(crate) fn read_to_raw_chunks<R: Read>(reader: R) -> errors::Result<Vec<RawChunk>> {
    let reader = BufReader::new(reader);

    let mut raw_chunks: Vec<RawChunk> = Vec::new();
    let mut file_header_found: bool = false;

    let mut content_iter = reader
        .bytes()
        .peekable()
        // TODO remove this unwrap, error properly? Or is this fine due to lazy evaluation?
        .map(|res| res.unwrap())
        .enumerate();

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

pub(crate) fn raw_chunks_to_chunks(raw_chunks: Vec<RawChunk>) -> errors::Result<Vec<Chunk>> {
    let mut chunks: Vec<Chunk> = vec![];

    //map channel IDs to format and channel counts from streamheader chunks to
    //be able to parse sampole chunks
    let mut stream_info_map = HashMap::<u32, StreamHeaderChunkInfo>::new();
    let mut stream_num_samples_map = HashMap::<u32, u64>::new();

    for raw_chunk in raw_chunks {
        //stream IDs are always the first 4 bytes.
        //if the chunk does not have a stream ID we can just ignore these. All
        //chunk content bytes are longer than 4 bytes anyway.
        let id_bytes = &raw_chunk.content_bytes[..4];
        let stream_id: u32 = LittleEndian::read_u32(id_bytes);
        match raw_chunk.tag {
            Tag::FileHeader => {
                let root = {
                    match Element::parse(raw_chunk.content_bytes.as_slice()) {
                        Ok(root) => root,
                        Err(err) => return Err(ParseChunkError::XMLParseError(err).into()),
                    }
                };

                let file_header_chunk = FileHeaderChunk {
                    version: parse_version(&root)?,
                    xml: root,
                };

                if file_header_chunk.version != 1.0 {
                    return Err(ParseChunkError::VersionNotSupportedError(file_header_chunk.version).into());
                }

                chunks.push(Chunk::FileHeaderChunk(file_header_chunk));
            }

            Tag::StreamHeader => {
                //first 4 bytes are stream id
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
                    channel_count: get_text_from_child(&root, "channel_count")?.parse().map_err(|err| {
                        ParseChunkError::BadElementError(format!("Error while parsing channel count: {}", err))
                    })?,
                    nominal_srate: Some(get_text_from_child(&root, "nominal_srate")?.parse().map_err(|err| {
                        ParseChunkError::BadElementError(format!("Error while parsing channel count: {}", err))
                    })?),
                    channel_format: match get_text_from_child(&root, "channel_format")?.to_lowercase().as_str() {
                        "in8" => Format::Int8,
                        "int16" => Format::Int16,
                        "int32" => Format::Int32,
                        "int64" => Format::Int64,
                        "float32" => Format::Float32,
                        "double64" => Format::Float64,
                        "string" => Format::String,
                        invalid => {
                            return Err(ParseChunkError::BadElementError(format!(
                                "Invalid stream format \"{}\"",
                                invalid
                            ))
                            .into())
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

                chunks.push(Chunk::StreamHeaderChunk(stream_header_chunk));
            }
            Tag::Samples => {
                //number of bytes used to represent the number of samples contained
                //in this chunk
                let num_samples_byte_num = &raw_chunk.content_bytes[4];

                //allow only valid options as per spec
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

                //vector of bytes which together form the number of samples
                let num_samples_bytes = &raw_chunk.content_bytes[5..(5 + num_samples_byte_num) as usize];

                //the actual number of samples
                let num_samples: u64 = LittleEndian::read_uint(num_samples_bytes, *num_samples_byte_num as usize);

                //TODO: bounds checks. probably use .get or something

                if stream_num_samples_map.contains_key(&stream_id) {
                    stream_num_samples_map
                        .insert(stream_id, stream_num_samples_map.get(&stream_id).unwrap() + num_samples);
                } else {
                    stream_num_samples_map.insert(stream_id, num_samples);
                }

                //for numeric values:
                //number of values = no. channels
                //size of a sample = 1 + (0 or 8) + no. channels * size of type
                //fuck, the timestamp thing makes it variable
                //why
                //realistically there will be timestamps for either all samples or
                //for none of them but the spec technically allows for other stuff
                //ffs

                let stream_info = stream_info_map
                    .get(&stream_id)
                    .ok_or(ParseChunkError::MissingHeaderError { stream_id })?;

                //option here because string doesn't have a constant size
                let type_size: Option<i32> = match stream_info.channel_format {
                    Format::Int8 => Some(1),
                    Format::Int16 => Some(2),
                    Format::Int32 => Some(4),
                    Format::Int64 => Some(8),
                    Format::Float32 => Some(4),
                    Format::Float64 => Some(8),
                    Format::String => None,
                };

                //offset:
                // 4 bytes for streamID
                // 1 byte for number of bytes for sample count
                // num_samples_byte_num for sample count
                let mut offset: usize = 4 + 1 + *num_samples_byte_num as usize;

                let mut samples: Vec<Sample> = Vec::with_capacity(num_samples as usize);

                //TODO is it worth having two loops only to not have to check
                //inside?
                //pro: performance? should test
                //cons: duplicate code
                if let Some(type_size) = type_size {
                    //constant size types
                    for _ in 0..num_samples {
                        let mut values: Vec<Value> = Vec::with_capacity(stream_info.channel_count as usize);
                        let timestamp: Option<f64> = extract_timestamp(&raw_chunk, &mut offset);

                        //values
                        for _ in 0..stream_info.channel_count {
                            let value_bytes = &raw_chunk.content_bytes[offset..(offset + type_size as usize)];
                            let value: Value = match stream_info.channel_format {
                                Format::Int8 => Value::Int8(value_bytes[0] as i8),
                                Format::Int16 => Value::Int16(LittleEndian::read_i16(value_bytes)),
                                Format::Int32 => Value::Int32(LittleEndian::read_i32(value_bytes)),
                                Format::Int64 => Value::Int64(LittleEndian::read_i64(value_bytes)),
                                Format::Float32 => Value::Float32(LittleEndian::read_f32(value_bytes)),
                                Format::Float64 => Value::Float64(LittleEndian::read_f64(value_bytes)),
                                Format::String => unreachable!(),
                            };

                            values.push(value);
                            offset += type_size as usize;
                        }

                        samples.push(Sample { timestamp, values });
                    }
                } else {
                    //strings
                    for _ in 0..num_samples {
                        let timestamp: Option<f64> = extract_timestamp(&raw_chunk, &mut offset);
                        let num_length_bytes: usize = raw_chunk.content_bytes[offset] as usize;
                        offset += 1; //for number of length bytes field
                        let value_length = match num_length_bytes {
                            1 | 4 | 8 => LittleEndian::read_uint(
                                &raw_chunk.content_bytes[offset..(offset + num_length_bytes)],
                                num_length_bytes,
                            ),
                            _ => {
                                let msg = format!("Error: Number of length bytes for this value are invalid. Expected 1, 4 or 8 but got {}", num_length_bytes);
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
                        let mut value_vec = Vec::with_capacity(1);
                        value_vec.push(value); // we need to put this into a vec with one element due to how the sample struct
                                               // works
                        samples.push(Sample {
                            timestamp,
                            values: value_vec,
                        });
                        offset += value_length; // for value field
                    }
                }

                let samples_chunk = Chunk::SamplesChunk(SamplesChunk { stream_id, samples });
                chunks.push(samples_chunk);
            }
            Tag::ClockOffset => {
                let collection_time: f64 = LittleEndian::read_f64(&raw_chunk.content_bytes[4..12]);
                let offset_value: f64 = LittleEndian::read_f64(&raw_chunk.content_bytes[12..20]);

                let clock_offset_chunk = Chunk::ClockOffsetChunk(ClockOffsetChunk {
                    stream_id,
                    collection_time,
                    offset_value,
                });
                chunks.push(clock_offset_chunk);
            }
            Tag::Boundary => chunks.push(Chunk::BoundaryChunk(BoundaryChunk {})),
            Tag::StreamFooter => {
                //first 4 bytes are stream id
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
                let measured_srate = opt_string_to_f64(measured_srate_str)?;

                let measured_srate = if let Some(val) = measured_srate {
                    val
                } else {
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
                };

                let stream_info = stream_info_map.get(&stream_id).unwrap(); // TODO error properly

                // if nominal_srate is given as zero (so it is None), measured_srate is also irrelevant
                let measured_srate = match stream_info.nominal_srate {
                    Some(_) => Some(measured_srate),
                    None => None,
                };

                // get_text_from_child(&root, "measured_srate")?.parse();

                let info = StreamFooterChunkInfo {
                    first_timestamp,
                    last_timestamp,
                    sample_count: get_text_from_child(&root, "sample_count")?.parse().map_err(|err| {
                        ParseChunkError::BadElementError(format!("Error while parsing sample count: {}", err))
                    })?,
                    measured_srate,
                };

                let stream_footer_chunk = Chunk::StreamFooterChunk(StreamFooterChunk {
                    stream_id,
                    info,
                    xml: root,
                });

                chunks.push(stream_footer_chunk);
            }
        }
    }

    return Ok(chunks);
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
