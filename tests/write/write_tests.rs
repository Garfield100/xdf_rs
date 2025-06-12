use paste::paste;
use std::{fs, vec};
use test_log::test;
use tracing::debug;
use zerocopy::{Immutable, IntoBytes};

use strict_num::{NonZeroPositiveF64, PositiveF64};
use xdf::{
    writer::{
        stream_format::{NumberFormat, StreamFormat},
        HasMetadataAndDesc, HasTimestamps, StreamInfo, XDFBuilder,
    },
    Values, XDFFile,
};

macro_rules! simple_num_test {
    ($t: ty) => {
        paste! {

        #[test]
        fn [<write_simple_ $t>]() {
            write_simple_num::<$t>();
        }
        }
    };
}

simple_num_test!(i8);
simple_num_test!(i16);
simple_num_test!(i32);
simple_num_test!(i64);
simple_num_test!(f32);
simple_num_test!(f64);

fn values_as_u64(val: &Values) -> Vec<u64> {
    match val {
        Values::Int8(v) => v.iter().map(|n| *n as u64).collect(),
        Values::Int16(v) => v.iter().map(|n| *n as u64).collect(),
        Values::Int32(v) => v.iter().map(|n| *n as u64).collect(),
        Values::Int64(v) => v.iter().map(|n| *n as u64).collect(),
        Values::Float32(v) => v.iter().map(|n| n.to_bits() as u64).collect(),
        Values::Float64(v) => v.iter().map(|n| n.to_bits()).collect(),
        Values::Strings(_) => {
            panic!("String values are not supported in this test");
        }
    }
}

fn to_u64<T: IntoBytes + Immutable>(value: T) -> u64 {
    let bytes = value.as_bytes();
    assert!(bytes.len() <= 8, "Value is too large to fit in u64");
    let mut out = [0u8; 8];
    out[..bytes.len()].copy_from_slice(bytes);
    u64::from_le_bytes(out)
}

fn write_simple_num<T: Clone + Copy + StreamFormat + NumberFormat + From<i8> + IntoBytes>() {
    let mut buffer = Vec::new();
    let mut writer = XDFBuilder::new().build(&mut buffer).unwrap();
    let stream_info = StreamInfo::new(2, Some(NonZeroPositiveF64::new(100.0).unwrap()));
    let mut stream = writer
        .add_stream::<T, HasTimestamps>(stream_info.clone())
        .name("Test Stream")
        .content_type("Test Content")
        .add_metadata_key("key1", "value1")
        .start_stream()
        .unwrap();

    let samples: Vec<[T; 2]> = vec![
        [T::from(1), T::from(2)],
        [T::from(3), T::from(4)],
        [T::from(5), T::from(6)],
    ];

    let timestamp = PositiveF64::new(1.0).unwrap();

    stream.write_samples(&samples, timestamp).unwrap();

    drop(stream);

    let parsed = XDFFile::from_bytes(&buffer).unwrap();
    debug!(?parsed);

    assert_eq!(parsed.version, 1.0);

    // test stream properties
    assert_eq!(parsed.streams.len(), 1);
    let stream = &parsed.streams[0];
    assert_eq!(stream.channel_count, 2);
    assert_eq!(stream.name.as_deref(), Some("Test Stream"));
    assert_eq!(stream.content_type.as_deref(), Some("Test Content"));
    assert_eq!(stream.header.get_child("key1").unwrap().get_text().unwrap(), "value1");
    assert_eq!(stream.samples.len(), samples.len());

    for (i, expected_sample) in samples.iter().enumerate() {
        assert_eq!(
            values_as_u64(&stream.samples[i].values),
            expected_sample.iter().map(|v| to_u64(*v)).collect::<Vec<_>>()
        );
        let expected_timestamp = timestamp.get() + i as f64 / stream_info.nominal_srate.unwrap().get();
        assert_eq!(stream.samples[i].timestamp.unwrap(), expected_timestamp);
    }
}

// TODO deduplicate

#[test]
fn write_simple_str_two_ch() {
    let mut buffer = Vec::new();
    let mut writer = XDFBuilder::new().build(&mut buffer).unwrap();
    let stream_info = StreamInfo::new(2, Some(NonZeroPositiveF64::new(100.0).unwrap()));
    let mut stream = writer
        .add_stream::<&str, HasTimestamps>(stream_info.clone())
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
    debug!(?parsed);

    assert_eq!(parsed.version, 1.0);

    // test stream properties
    assert_eq!(parsed.streams.len(), 1);
    let stream = &parsed.streams[0];
    assert_eq!(stream.channel_count, 2);
    assert_eq!(stream.name.as_deref(), Some("Test Stream"));
    assert_eq!(stream.content_type.as_deref(), Some("Test Content"));
    assert_eq!(stream.header.get_child("key1").unwrap().get_text().unwrap(), "value1");
    assert_eq!(stream.samples.len(), samples.len());

    for (i, expected_sample) in samples.iter().enumerate() {
        // assert_eq!(stream.samples[i].values, sample);
        match &stream.samples[i].values {
            Values::Strings(strings) => {
                assert_eq!(strings.as_slice(), expected_sample)
            }
            _ => panic!("Wrong value type for string test"),
        }
        let expected_timestamp = timestamp.get() + i as f64 / stream_info.nominal_srate.unwrap().get();
        assert_eq!(stream.samples[i].timestamp.unwrap(), expected_timestamp);
    }
}

#[test]
fn write_simple_str_one_ch() {
    let mut buffer = Vec::new();
    let mut writer = XDFBuilder::new().build(&mut buffer).unwrap();
    let stream_info = StreamInfo::new(1, Some(NonZeroPositiveF64::new(100.0).unwrap()));
    let mut stream = writer
        .add_stream::<&str, HasTimestamps>(stream_info.clone())
        .name("Test Stream")
        .content_type("Test Content")
        .add_metadata_key("key1", "value1")
        .start_stream()
        .unwrap();

    let samples: Vec<[&str; 1]> = vec![["one"], ["two"], ["🦀"]];

    let timestamp = PositiveF64::new(1.0).unwrap();

    stream.write_samples(&samples, timestamp).unwrap();

    drop(stream);

    println!("buffer: {:?}", String::from_utf8_lossy(&buffer));
    println!("buffer: {:?}", &buffer);

    // fs::write("str_test.xdf", &buffer).expect("Could not write file");

    let parsed = XDFFile::from_bytes(&buffer).unwrap();
    debug!(?parsed);

    assert_eq!(parsed.version, 1.0);

    // test stream properties
    assert_eq!(parsed.streams.len(), 1);
    let stream = &parsed.streams[0];
    assert_eq!(stream.channel_count, 1);
    assert_eq!(stream.name.as_deref(), Some("Test Stream"));
    assert_eq!(stream.content_type.as_deref(), Some("Test Content"));
    assert_eq!(stream.header.get_child("key1").unwrap().get_text().unwrap(), "value1");
    assert_eq!(stream.samples.len(), samples.len());

    for (i, expected_sample) in samples.iter().enumerate() {
        // assert_eq!(stream.samples[i].values, sample);
        match &stream.samples[i].values {
            Values::Strings(strings) => {
                assert_eq!(strings.as_slice(), expected_sample)
            }
            _ => panic!("Wrong value type for string test"),
        }
        let expected_timestamp = timestamp.get() + i as f64 / stream_info.nominal_srate.unwrap().get();
        assert_eq!(stream.samples[i].timestamp.unwrap(), expected_timestamp);
    }
}
