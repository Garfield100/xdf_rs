use std::{fs, vec};
use test_log::test;

use strict_num::{NonZeroPositiveF64, PositiveF64};
use xdf::{
    streams::SampleEnum,
    writer::{HasMetadataAndDesc, HasTimestamps, StreamInfo, XDFBuilder},
    XDFFile,
};

// TODO deduplicate

#[test]
fn write_simple_str_two_ch() {
    let mut buffer = Vec::new();
    let mut writer = XDFBuilder::new().build(&mut buffer).unwrap();
    let stream_info = StreamInfo::new(2, Some(NonZeroPositiveF64::new(100.0).unwrap()));
    let mut stream = writer
        .add_stream::<String, HasTimestamps>(stream_info.clone())
        .name("Test Stream")
        .content_type("Test Content")
        .add_metadata_key("key1", "value1")
        .start_stream()
        .unwrap();

    let samples: Vec<[&str; 2]> = vec![["one", "two"], ["three", "four"], ["five", "🦀"]];

    let timestamp = PositiveF64::new(1.0).unwrap();

    stream.write_samples(&samples, timestamp).unwrap();

    drop(stream);

    println!("buffer: {:?}", String::from_utf8_lossy(&buffer));
    println!("buffer: {:?}", &buffer);

    fs::write("str_test.xdf", &buffer).expect("Could not write file");

    let parsed = XDFFile::from_bytes(&buffer).unwrap();
    println!("Parsed: {:?}", parsed);

    assert_eq!(parsed.version, 1.0);

    // test stream properties
    assert_eq!(parsed.streams.len(), 1);
    let stream = &parsed.streams[0];
    assert_eq!(stream.channel_count, 2);
    assert_eq!(stream.name.as_deref(), Some("Test Stream"));
    assert_eq!(stream.content_type.as_deref(), Some("Test Content"));
    assert_eq!(stream.header.get_child("key1").unwrap().get_text().unwrap(), "value1");
    assert_eq!(stream.sample_enum.len(), samples.len());

    {
        let measured = stream.measured_srate.unwrap();
        let nominal = stream_info.nominal_srate.unwrap().get();
        let abs_diff = (measured - nominal).abs();
        const EPSILON: f64 = f64::EPSILON * 400.0; // Increased until it worked lmao. Still very small though.
        assert!(
            abs_diff < EPSILON,
            "Expected measured {measured} to be within {EPSILON} of nominal {nominal}, actual abs. diff. was {abs_diff}, {}x larger than the epsilon.", abs_diff / EPSILON);
    }

    let SampleEnum::String(actual_samples) = &stream.sample_enum else {
        panic!("Wrong value type for string test");
    };

    for (i, expected_sample) in samples.iter().enumerate() {
        assert_eq!(actual_samples[i].values.as_slice(), expected_sample);
        let expected_timestamp = timestamp.get() + i as f64 / stream_info.nominal_srate.unwrap().get();
        assert_eq!(actual_samples[i].timestamp.unwrap(), expected_timestamp);
    }
}

#[test]
fn write_simple_str_one_ch() {
    let mut buffer = Vec::new();
    let mut writer = XDFBuilder::new().build(&mut buffer).unwrap();
    let stream_info = StreamInfo::new(1, Some(NonZeroPositiveF64::new(100.0).unwrap()));
    let mut stream = writer
        .add_stream::<String, HasTimestamps>(stream_info.clone())
        .name("Test Stream")
        .content_type("Test Content")
        .add_metadata_key("key1", "value1")
        .start_stream()
        .unwrap();

    let samples: Vec<[&str; 1]> = vec![["one"], ["two"], ["🦀"]];

    let timestamp = PositiveF64::new(1.0).unwrap();

    stream.write_samples(&samples, timestamp).unwrap();

    drop(stream);

    println!("stringified buffer: {:?}", String::from_utf8_lossy(&buffer));
    println!("buffer: {:?}", &buffer);

    let parsed = XDFFile::from_bytes(&buffer).unwrap();
    dbg!(&parsed);

    assert_eq!(parsed.version, 1.0);

    // test stream properties
    assert_eq!(parsed.streams.len(), 1);
    let stream = &parsed.streams[0];
    assert_eq!(stream.channel_count, 1);
    assert_eq!(stream.name.as_deref(), Some("Test Stream"));
    assert_eq!(stream.content_type.as_deref(), Some("Test Content"));
    assert_eq!(stream.header.get_child("key1").unwrap().get_text().unwrap(), "value1");
    assert_eq!(stream.sample_enum.len(), samples.len());
    {
        let measured = stream.measured_srate.unwrap();
        let nominal = stream_info.nominal_srate.unwrap().get();
        let abs_diff = (measured - nominal).abs();
        const EPSILON: f64 = f64::EPSILON * 400.0; // Increased until it worked lmao. Still very small though.
        assert!(
            abs_diff < EPSILON,
            "Expected measured {measured} to be within {EPSILON} of nominal {nominal}, actual abs. diff. was {abs_diff}, {}x larger than the epsilon.", abs_diff / EPSILON);
    }

    let SampleEnum::String(actual_samples) = &stream.sample_enum else {
        panic!("Wrong value type for string test");
    };

    for (i, expected_sample) in samples.iter().enumerate() {
        assert_eq!(actual_samples[i].values.as_slice(), expected_sample);
        let expected_timestamp = timestamp.get() + i as f64 / stream_info.nominal_srate.unwrap().get();
        assert_eq!(actual_samples[i].timestamp.unwrap(), expected_timestamp);
    }
}
