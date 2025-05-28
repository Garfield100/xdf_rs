use xmltree::{Element, XMLNode};

use crate::writer::SharedState;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use crate::writer::StreamInfo;

use super::{
    error::XDFWriterError, stream_format::StreamFormat, stream_writer::StreamWriter, timestamp::Timestamped,
    xdf_builder::xml_add_child_overwrite, StreamID,
};

pub struct StreamBuilder<W: Write, F: StreamFormat, T: Timestamped> {
    pub(crate) id: StreamID,
    pub(crate) info: StreamInfo,
    pub(crate) state: Arc<Mutex<SharedState<W>>>,
    metadata: Element,
    desc: Element,
    _timestamp_marker: std::marker::PhantomData<T>,
    _format_marker: std::marker::PhantomData<F>,
}

impl<W: Write, F: StreamFormat, T: Timestamped> StreamBuilder<W, F, T> {
    pub(crate) fn new(id: StreamID, info: StreamInfo, state: Arc<Mutex<SharedState<W>>>) -> Self {
        Self {
            id,
            info,
            state,
            metadata: Element::new("info"),
            desc: Element::new("desc"),
            _timestamp_marker: std::marker::PhantomData,
            _format_marker: std::marker::PhantomData,
        }
    }

    pub fn start_stream(self) -> Result<StreamWriter<W, F, T>, XDFWriterError> {
        // ensure srate is finite

        let mut metadata = self.metadata.clone();
        let desc = self.desc.clone();

        // overwrite the desc
        let _ = metadata.take_child("desc");
        metadata.children.push(XMLNode::Element(desc));

        let channel_count = self.info.channel_count.to_string();
        let channel_format: String = F::get_format().into();
        let nominal_srate = self
            .info
            .nominal_srate
            .map_or_else(|| "0".to_string(), |s| s.to_string());

        // overwrite channel_count, nominal_srate, and channel_format using StreamInfo
        xml_add_child_overwrite(&mut metadata, "channel_count", channel_count);
        xml_add_child_overwrite(&mut metadata, "nominal_srate", nominal_srate);
        xml_add_child_overwrite(&mut metadata, "channel_format", channel_format);

        {
            let mut state_lock = self.state.lock()?;
            state_lock.write_helper.write_stream_header(self.id, &metadata)?;
        }

        Ok(StreamWriter {
            state: self.state,
            info: self.info,
            id: self.id,
            _timestamp_marker: std::marker::PhantomData::<T>,
            _format_marker: std::marker::PhantomData::<F>,
        })
    }

    pub fn name<S: Into<String>>(self, name: S) -> Self {
        self.add_metadata_key("name", name)
    }

    pub fn content_type<S: Into<String>>(self, content_type: S) -> Self {
        self.add_metadata_key("type", content_type)
    }

    pub fn add_desc_key<S: Into<String>>(mut self, key: &str, value: S) -> Self {
        xml_add_child_overwrite(&mut self.desc, key, value);
        self
    }

    /// Adds a key-value pair to the stream's XML metadata.
    /// Note that the following 3 fields will be overwritten once the builder is finalised:`channel_count`, `nominal_srate`, `channel_format`.
    // TODO example
    pub fn add_metadata_key<S: Into<String>>(mut self, key: &str, value: S) -> Self {
        xml_add_child_overwrite(&mut self.metadata, key, value);
        self
    }

    /// Returns a mutable reference to an XML Element which will be used to generate the [stream header](https://github.com/sccn/xdf/wiki/Specifications#streamheader-chunk)'s XML metadata.
    /// See other methods for more convenient ways of modifying this.
    /// This direct access is only really necessary if you e.g. wish to add nested elements etc.
    /// Do not rely on this containing anything at all
    /// Also note that the following 3 fields will be overwritten once the builder is finalised: `channel_count`, `nominal_srate`, `channel_format`.
    pub fn get_metadata_mut(&mut self) -> &mut Element {
        &mut self.metadata
    }
}
