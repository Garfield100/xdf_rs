use byteorder::{ByteOrder, LittleEndian};
use xmltree::Element;

use crate::{chunk_structs::RawChunk, errors::ParseChunkError};

pub(crate) fn parse_version(root: &Element) -> Result<f32, ParseChunkError> {
    let version_element = match root.get_child("version") {
        Some(child) => child,

        //XML does not contain the tag "version"
        None => return Err(ParseChunkError::BadElementError("version".to_string())),
    };

    let version_str = {
        match version_element.get_text() {
            Some(val) => val,

            //the version tag exists but it is empty
            None => return Err(ParseChunkError::BadElementError("version".to_string())),
        }
    };

    let version = {
        match version_str.parse::<f32>() {
            Ok(t) => t,

            //the version text could not be parsed into a float
            Err(_e) => {
                return Err(ParseChunkError::BadElementError("version".to_string()));
            }
        }
    };

    return Ok(version);
}

pub(crate) fn get_text_from_child(root: &Element, child_name: &str) -> Result<String, ParseChunkError> {
    Ok(root
        .get_child(child_name)
        .ok_or(ParseChunkError::BadElementError(child_name.to_string()))?
        .get_text()
        .ok_or(ParseChunkError::BadElementError(child_name.to_string()))?
        .to_string())
}

pub(crate) fn opt_string_to_f64(opt_string: Option<String>) -> Result<Option<f64>, ParseChunkError> {
    match opt_string {
        Some(val_str) => {
            let val_res = val_str.parse::<f64>();
            match val_res {
                Ok(val) => Ok(Some(val)),
                Err(err) => Err(ParseChunkError::BadElementError(format!(
                    "Error while parsing {}: {}",
                    val_str, err
                ))),
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn extract_timestamp(raw_chunk: &RawChunk, offset: &mut usize) -> Option<f64> {
    let timestamp: Option<f64>;
    if raw_chunk.content_bytes[*offset] == 8 {
        //we have a timestamp
        timestamp = Some(LittleEndian::read_f64(
            &raw_chunk.content_bytes[(*offset + 1)..(*offset + 9)],
        ));
        *offset += 9;
    } else {
        //no timestamp
        debug_assert_eq!(raw_chunk.content_bytes[*offset], 0);
        timestamp = None;
        *offset += 1;
    }

    return timestamp;
}
