// API design and builder pattern inspired by the CSV crate
use std::io::{self, Write};

use stream_handle::{StreamFormat, StreamHandle};

mod stream_handle;
mod xdf_builder;

#[derive(thiserror::Error, Debug)]
pub enum XDFWriterError {
    #[error(transparent)]
    XMLTree(#[from] xmltree::Error),
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Conversion(#[from] std::num::TryFromIntError),
}

pub struct XDFMeta {
    pub description: String,
    pub author: String,
    pub date: String,
}

struct FooterInfo {
    first_timestamp: Option<f64>,
    last_timestamp: Option<f64>,
    sample_count: usize,
}

pub struct XDFWriter<Dest: Write> {
    writer: Dest,
    footer_info: Vec<FooterInfo>,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    channel_count: usize,
    nominal_srate: Option<f64>,
    name: String,
    content_type: String,
}

impl<Dest: Write> XDFWriter<Dest> {
    pub(crate) fn new(writer: Dest) -> Self {
        // the specification suggests ordinal numbers starting at 1
        Self {
            writer,
            footer_info: Vec::new(),
        }
    }

    pub fn add_stream<T: StreamFormat>(&mut self, stream_info: StreamInfo) -> Result<StreamHandle<T>, XDFWriterError> {
        self.footer_info.push(FooterInfo {
            first_timestamp: None,
            last_timestamp: None,
            sample_count: 0,
        });

        // Spec says to start at 1, so get the length after pushing
        let stream_id = u32::try_from(self.footer_info.len())?;

        let handle = StreamHandle::new(stream_id, stream_info);
        self.writer.write_all(&handle.chunk_bytes()?)?;

        Ok(handle)
    }

    pub fn write_num_samples<T: StreamFormat>(
        &mut self,
        handle: &StreamHandle<T>,
        sample: &[&[T]],
        timestamp: Option<f64>,
    ) -> Result<(), XDFWriterError> {
        assert!(
            sample.len() == handle.stream_info.channel_count,
            "Data length ({}) does not match stream channel count ({})",
            sample.len(),
            handle.stream_info.channel_count
        );

        // let mut bytes = Vec::new();
        // let samples_bytes

        todo!("Implement writing number samples")
    }

    pub fn write_string_sample<T: AsRef<str>>(
        &mut self,
        handle: &StreamHandle<&str>,
        sample: T,
        timestamp: Option<f64>,
    ) -> Result<(), XDFWriterError> {
        let len = sample.as_ref().len();

        // let mut bytes = Vec::new();

        todo!("implement writing string samples")
    }
}

#[test]
fn test_write_sample_int() {
    let mut buffer = Vec::new();
    let mut writer = XDFWriter::new(&mut buffer);

    let stream_info = StreamInfo {
        channel_count: 2,
        nominal_srate: Some(100.0),
        name: "Integer Stream".to_string(),
        content_type: "EEG".to_string(),
    };

    let handle = writer.add_stream::<i32>(stream_info).unwrap();

    let sample = [1, 2];
    writer.write_num_samples(&handle, &[&sample], None).unwrap();
    assert_ne!(buffer.len(), 0); // TODO write better test, this is mostly for type checking
}

#[test]
fn test_write_sample_string() {
    let mut buffer = Vec::new();
    let mut writer = XDFWriter::new(&mut buffer);

    let stream_info = StreamInfo {
        channel_count: 1,
        nominal_srate: None,
        name: "String Stream".to_string(),
        content_type: "Marker".to_string(),
    };

    let handle = writer.add_stream::<&str>(stream_info).unwrap();

    let sample = "Hello 🦀!";
    writer.write_string_sample(&handle, sample, None).unwrap();
    assert_ne!(buffer.len(), 0); // TODO write better test, this is mostly for type checking
}
