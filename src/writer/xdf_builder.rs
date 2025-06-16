use std::io::Write;

use xmltree::{Element, XMLNode};

use super::{error::XDFWriterError, WriteHelper, XDFWriter};

pub struct XDFBuilder {
    file_header: Element,

    /// Space for optional meta-information. [See the page on Metadata in the XDF wiki](https://github.com/sccn/xdf/wiki/Meta-Data)
    pub desc: Element,
}

impl Default for XDFBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// pub struct XDFMeta {
//     pub description: String,
//     pub author: String,
//     pub date: String,
// }

impl XDFBuilder {
    pub fn new() -> Self {
        let file_header = Element::new("info");
        let desc = Element::new("desc");

        XDFBuilder { file_header, desc }
    }

    pub fn build<W: Write>(mut self, writer: W) -> Result<XDFWriter<W>, XDFWriterError> {
        // overwrite `version` and `xdf_crate_version` in case they were set before
        self.ensure_fields();

        let mut write_helper = WriteHelper { writer };

        // add the <desc> description to the file header
        self.file_header.children.push(XMLNode::Element(self.desc));

        write_helper.write_file_header(&self.file_header)?;

        Ok(XDFWriter::new(write_helper))
    }

    fn ensure_fields(&mut self) {
        xml_add_child_overwrite(&mut self.file_header, "version", "1.0");
        let this_crate_version =
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown, missing CARGO_PKG_VERSION env at build time");

        xml_add_child_overwrite(
            &mut self.file_header,
            "xdf_crate_version",
            this_crate_version.to_string(),
        );
    }
}

pub trait HasMetadataAndDesc: Sized {
    /// Returns a mutable reference to an XML Element which forms the Header's metadata.
    /// See other methods for more convenient ways of modifying this.
    /// This direct access is only really necessary if you e.g. wish to add nested elements etc.
    /// Note that certain elements (such as, for example, `<version>`) will be overwritten once the builder is finalised.
    /// See the respective struct's docs for specifics.
    /// Store things in the `<desc>` element using the appropriate methods to avoid this.
    fn get_metadata_mut(&mut self) -> &mut Element;

    /// Returns a mutable reference to an XML Element which forms the Header's description (`<desc>`).
    /// See other methods for more convenient ways of modifying this.
    /// No fields in the `<desc>` tag will be overwritten by the builder.
    fn get_desc_mut(&mut self) -> &mut Element;

    /// Adds a key-value pair to the top level XML metadata.
    /// Note that certain elements (such as, for example, `<version>`) will be overwritten once the builder is finalised.
    /// See the respective struct's docs for specifics.
    /// Store things in the `<desc>` element using the appropriate methods to avoid this.
    // TODO example
    fn add_metadata_key<S: Into<String>>(mut self, key: &str, value: S) -> Self {
        xml_add_child_overwrite(self.get_metadata_mut(), key, value);
        self
    }

    /// Adds a key-value pair to the <desc> XML tag.
    /// No fields in the <desc> tag will be overwritten by the builder.
    // TODO example
    fn add_desc_key<S: Into<String>>(mut self, key: &str, value: S) -> Self {
        xml_add_child_overwrite(self.get_desc_mut(), key, value);
        self
    }
}

impl HasMetadataAndDesc for XDFBuilder {
    fn get_metadata_mut(&mut self) -> &mut Element {
        &mut self.file_header
    }

    fn get_desc_mut(&mut self) -> &mut Element {
        &mut self.desc
    }
}

pub(crate) fn xml_add_child_unchecked<T: Into<String>>(elem: &mut Element, key: &str, value: T) {
    let mut child = Element::new(key);
    let value = XMLNode::Text(value.into());

    child.children.push(value);

    let child = XMLNode::Element(child);

    elem.children.push(child);
}

pub(crate) fn xml_add_child_overwrite<T: Into<String>>(elem: &mut Element, key: &str, value: T) {
    let _old_child = elem.take_child(key);
    xml_add_child_unchecked(elem, key, value);
}

#[test]
fn test_file_header() {
    let mut buf: Vec<u8> = Vec::new();
    let builder = XDFBuilder::new();
    let _writer = builder.build(&mut buf).expect("Failed to build writer");

    println!("{}", String::from_utf8_lossy(&buf));

    // println!("{:?}", &buf);xdf_crate_version
    // println!("{}", String::from_utf8_lossy(&buf));

    assert!(buf.starts_with(b"XDF:"), "No magic number");
    assert_eq!(&buf[4..6], [1, 127], "Incorrect length"); // technically due to the XML stuff this can change a bit and still be valid, depending on what the XML library outputs
    assert_eq!(&buf[6..8], [1, 0], "Incorrect chunk tag");

    let parsed_xml = Element::parse(&buf[8..]).expect("Header not valid XML");

    assert_eq!(parsed_xml.name, "info");
    assert!(
        parsed_xml
            .children
            .iter()
            .any(|c| c.as_element().unwrap().name == "version"),
        "No version element"
    );

    assert_eq!(
        parsed_xml
            .children
            .iter()
            .find(|c| c.as_element().unwrap().name == "xdf_crate_version")
            .expect("No xdf_crate_version element")
            .as_element()
            .unwrap()
            .children[0]
            .as_text()
            .expect("Child should be text"),
        env!("CARGO_PKG_VERSION")
    );

    // todo!()
}
