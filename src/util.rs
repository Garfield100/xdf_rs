use byteorder::{ByteOrder, LittleEndian};
use error_chain::bail;
use xmltree::Element;

use crate::{chunk_structs::RawChunk, errors::*};

pub(crate) fn parse_version(root: &Element) -> Result<f32> {
    let version_element = match root.get_child("version") {
        Some(child) => child,

        //XML does not contain the tag "version"
        None => bail!(Error::from("\"version\" XML tag not found")
            .chain_err(|| ErrorKind::BadXMLElementError("version".to_string()))),
    };

    let version_str = {
        match version_element.get_text() {
            Some(val) => val,

            //the version tag exists but it is empty
            None => bail!(Error::from("Empty \"version\" XML tag")
                .chain_err(|| ErrorKind::BadXMLElementError("version".to_string()))),
        }
    };

    let version = {
        match version_str.parse::<f32>() {
            Ok(t) => t,

            //the version text could not be parsed into a float
            Err(e) => bail!(Error::with_chain(
                e,
                ErrorKind::BadXMLElementError("version".to_string())
            )),
        }
    };

    Ok(version)
}

pub(crate) fn get_text_from_child(root: &Element, child_name: &str) -> Result<String> {
    Ok(root
        .get_child(child_name)
        .ok_or(ErrorKind::BadXMLElementError(child_name.to_string()))?
        .get_text()
        .ok_or(ErrorKind::BadXMLElementError(child_name.to_string()))?
        .to_string())
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

    timestamp
}

#[test]
fn test_extract_timestamp_none() {
    let mut offset = 0;
    let raw_chunk = RawChunk {
        tag: crate::chunk_structs::Tag::StreamHeader,
        content_bytes: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
    };

    let timestamp = extract_timestamp(&raw_chunk, &mut offset);
    assert_eq!(timestamp, None);
    assert_eq!(offset, 1);
}

#[test]
fn test_extract_timestamp_some() {
    let mut offset = 0;
    let raw_chunk = RawChunk {
        tag: crate::chunk_structs::Tag::StreamHeader,
        //5.1 = 0x 40 14 66 66 66 66 66 66
        content_bytes: vec![8, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x14, 0x40],
    };

    let timestamp = extract_timestamp(&raw_chunk, &mut offset);
    assert_eq!(timestamp, Some(5.09999999999999964472863211995_f64));
    assert_eq!(offset, 9);
}

#[test]
fn test_parse_version() {
    let mut root = Element::new("root");
    let mut version_element = Element::new("version");
    version_element.children.push(xmltree::XMLNode::Text("1.0".to_string()));
    root.children.push(xmltree::XMLNode::Element(version_element));

    let result = parse_version(&root);
    assert_eq!(result.unwrap(), 1.0);
}

#[test]
fn test_parse_version_missing_tag() {
    let root = Element::new("root");

    let result = parse_version(&root);
    assert!(result.is_err());
}

#[test]
fn test_parse_version_empty_tag() {
    let mut root = Element::new("root");
    let version_element = Element::new("version");
    root.children.push(xmltree::XMLNode::Element(version_element));

    let result = parse_version(&root);
    assert!(result.is_err());
}

#[test]
fn test_get_text_from_child() {
    let mut root = Element::new("root");
    let mut child_element = Element::new("child");
    let child_text = "value".to_string();

    child_element.children.push(xmltree::XMLNode::Text(child_text.clone()));

    root.children.push(xmltree::XMLNode::Element(child_element));

    let result = get_text_from_child(&root, "child");
    assert_eq!(result.unwrap(), child_text);
}

#[test]
fn test_get_text_from_child_missing_child() {
    let root = Element::new("root");

    let result = get_text_from_child(&root, "child");
    assert!(result.is_err());
}

#[test]
fn test_get_text_from_child_empty_child() {
    let mut root = Element::new("root");
    let child_element = Element::new("child");
    root.children.push(xmltree::XMLNode::Element(child_element));

    let result = get_text_from_child(&root, "child");
    assert!(result.is_err());
}
