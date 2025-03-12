use std::fmt::Display;

use thiserror::Error;
use xmltree::Element;

use crate::errors::{ParseError, XDFError, XMLError};

pub(crate) fn parse_version(root: &Element) -> Result<f32, XDFError> {
    let version_element: &Element = root
        .get_child("version")
        .ok_or(XMLError::BadElement("version".to_string()))?;

    let version_str = version_element
        .get_text()
        .ok_or(XMLError::BadElement("version".to_string()))?;

    let version = version_str.parse::<f32>().map_err(ParseError::from)?;

    Ok(version)
}

pub(crate) fn get_text_from_child(root: &Element, child_name: &str) -> Result<String, XDFError> {
    Ok(root
        .get_child(child_name)
        .ok_or(XMLError::BadElement(child_name.to_string()))?
        .get_text()
        .ok_or(XMLError::BadElement(child_name.to_string()))?
        .to_string())
}

// #[derive(Debug, Error)]
// pub(crate) struct NotFiniteError();
// impl Display for NotFiniteError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         writeln!(f, "The provided f64 is not finite.")
//     }
// }

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub(crate) struct FiniteF64(f64);

impl FiniteF64 {
    pub(crate) fn new(float: f64) -> Option<Self> {
        if float.is_finite() {
            Some(Self(float))
        } else {
            None
        }
    }

    pub(crate) const fn zero() -> Self {
        Self(0.0)
    }
}

impl Eq for FiniteF64 {}

#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for FiniteF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // finite, therefore safe to unwrap
        self.partial_cmp(other)
            .expect("FiniteF64 comparison failed, please file an issue in xdf_rs")
    }
}

// impl TryFrom<f64> for FiniteF64 {
//     type Error = NotFiniteError;

//     fn try_from(value: f64) -> Result<Self, Self::Error> {
//         Self::new(value).ok_or(NotFiniteError())
//     }
// }

#[test]
#[allow(clippy::float_cmp)] // we're testing the version parses correctly, an exact comparison is fine
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
