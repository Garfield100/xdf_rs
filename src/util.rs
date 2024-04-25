use xmltree::Element;

use crate::errors::XDFError;

pub(crate) fn parse_version(root: &Element) -> Result<f32, XDFError> {
    let version_element: &Element = root
        .get_child("version")
        .ok_or(XDFError::BadXMLElementError("version".to_string()))?;

    let version_str = version_element
        .get_text()
        .ok_or(XDFError::BadXMLElementError("version".to_string()))?;

    let version = version_str.parse::<f32>()?;

    Ok(version)
}

pub(crate) fn get_text_from_child(root: &Element, child_name: &str) -> Result<String, XDFError> {
    Ok(root
        .get_child(child_name)
        .ok_or(XDFError::BadXMLElementError(child_name.to_string()))?
        .get_text()
        .ok_or(XDFError::BadXMLElementError(child_name.to_string()))?
        .to_string())
}

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
