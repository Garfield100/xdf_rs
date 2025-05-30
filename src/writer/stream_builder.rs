use xmltree::{Element, XMLNode};

use crate::writer::SharedState;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use crate::writer::StreamInfo;

use super::{
    error::XDFWriterError,
    stream_format::StreamFormat,
    stream_writer::StreamWriter,
    timestamp::TimestampTrait,
    xdf_builder::{xml_add_child_overwrite, HasMetadataAndDesc},
    StreamID,
};

///
///
/// Overwritten top-level XML elements: `channel_count`, `nominal_srate`, `channel_format`.
pub struct StreamBuilder<W: Write, F: StreamFormat, T: TimestampTrait> {
    pub(crate) id: StreamID,
    pub(crate) info: StreamInfo,
    pub(crate) state: Arc<Mutex<SharedState<W>>>,
    metadata: Element,
    desc: Element,
    _timestamp_marker: std::marker::PhantomData<T>,
    _format_marker: std::marker::PhantomData<F>,
}

impl<W: Write, F: StreamFormat, T: TimestampTrait> StreamBuilder<W, F, T> {
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
            first_timestamp: None,
            last_timestamp: None,
            num_samples_written: 0,
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
}

impl<W: Write, F: StreamFormat, T: TimestampTrait> HasMetadataAndDesc for StreamBuilder<W, F, T> {
    fn get_metadata_mut(&mut self) -> &mut Element {
        &mut self.metadata
    }

    fn get_desc_mut(&mut self) -> &mut Element {
        &mut self.desc
    }
}
