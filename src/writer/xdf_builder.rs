use std::io::{self, Write};

use xmltree::{Element, XMLNode};

use super::XDFWriter;

#[derive(thiserror::Error, Debug)]
pub enum XDFBuilderError {
    #[error(transparent)]
    XMLTree(#[from] xmltree::Error),

    #[error(transparent)]
    IO(#[from] io::Error),
}

pub struct XDFBuilder {
    file_header: Element,

    /// Space for optional meta-information. [See the page on Metadata in the XDF wiki](https://github.com/sccn/xdf/wiki/Meta-Data)
    pub desc: Element,
}

impl XDFBuilder {
    pub fn new() -> Self {
        let mut file_header = Element::new("info");
        file_header.attributes.insert("version".to_string(), "1.0".to_string());
        let this_crate_version =
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown, missing CARGO_PKG_VERSION env  at build time");
        file_header
            .attributes
            .insert("xdf_crate_version".to_string(), this_crate_version.to_string());

        // add empty desc tag to contain [optional metadata](https://github.com/sccn/xdf/wiki/Meta-Data)
        let desc = Element::new("desc");

        XDFBuilder { file_header, desc }
    }

    pub fn build<Dest: Write>(mut self, mut writer: Dest) -> Result<XDFWriter<Dest>, XDFBuilderError> {
        // add the description to the file header
        self.file_header.children.push(XMLNode::Element(self.desc));

        // write the file header
        self.file_header.write(&mut writer)?;
        writer.flush()?;

        Ok(XDFWriter::new(writer))
    }
}
