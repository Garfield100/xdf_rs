use std::vec;

use strict_num::{NonZeroPositiveF64, PositiveF64};
use xdf::{
    writer::{HasMetadataAndDesc, HasTimestamps, StreamInfo, XDFBuilder},
    XDFFile,
};

#[test]
fn write_simple() {
    let mut buffer = Vec::new();
    let mut writer = XDFBuilder::new().build(&mut buffer).unwrap();
    let stream_info = StreamInfo::new(2, Some(NonZeroPositiveF64::new(100.0).unwrap()));
    let mut stream = writer
        .add_stream::<i32, HasTimestamps>(stream_info)
        .name("Test Stream")
        .content_type("Test Content")
        .add_metadata_key("key1", "value1")
        .start_stream()
        .unwrap();

    let samples: Vec<&[i32]> = vec![&[1, 2], &[3, 4], &[5, 6]];
    let timestamp = PositiveF64::new(1.0).unwrap();

    stream.write_samples(&samples, timestamp).unwrap();

    drop(stream);

    let parsed = XDFFile::from_bytes(&buffer).unwrap();
    println!("Parsed XDFFile: {:#?}", parsed);

    // write to file
}
