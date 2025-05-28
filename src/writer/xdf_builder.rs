use std::io::{Write};

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

impl XDFBuilder {
    pub fn new() -> Self {
        let mut file_header = Element::new("info");
        xml_add_child_unchecked(&mut file_header, "version", "1.0");

        let this_crate_version =
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown, missing CARGO_PKG_VERSION env  at build time");

        xml_add_child_unchecked(&mut file_header, "xdf_crate_version", this_crate_version.to_string());

        // add empty desc tag to contain [optional metadata](https://github.com/sccn/xdf/wiki/Meta-Data)
        let desc = Element::new("desc");

        XDFBuilder { file_header, desc }
    }

    pub fn build<W: Write>(mut self, writer: W) -> Result<XDFWriter<W>, XDFWriterError> {
        let mut write_helper = WriteHelper { writer };

        // add the <desc> description to the file header
        self.file_header.children.push(XMLNode::Element(self.desc));

        write_helper.write_file_header(&self.file_header)?;

        Ok(XDFWriter::new(write_helper))
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
