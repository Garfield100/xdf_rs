use std::{fs, path::PathBuf};

use xdf::{Format, Value, XDFFile};

#[test]
fn read_minimal_xdf() {
    let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file_path.push("example-files/minimal.xdf");
    let reader = fs::File::open(file_path).unwrap();
    let xdf_file = XDFFile::from_reader(reader).unwrap();

    //must be sorted
    let stream_ids: [u32; 2] = [0, 0x02C0FFEE];

    assert_eq!(xdf_file.header.name, "info");

    assert_eq!(xdf_file.streams.keys().len(), stream_ids.len());
    let mut read_ids = xdf_file.streams.keys().map(|id| *id).collect::<Vec<u32>>();
    let mut expected_ids = stream_ids.clone();
    read_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(read_ids.as_slice(), stream_ids);

    let first_stream = xdf_file.streams.get(&stream_ids[0]).unwrap();
    let second_stream = xdf_file.streams.get(&stream_ids[1]).unwrap();

    let expected_first_samples = vec![
        xdf::Sample {
            timestamp: Some(5.1),
            values: vec![Value::Int16(192), Value::Int16(255), Value::Int16(238)],
        },
        xdf::Sample {
            timestamp: Some(5.2),
            values: vec![Value::Int16(12), Value::Int16(22), Value::Int16(32)],
        },
        xdf::Sample {
            timestamp: Some(5.3),
            values: vec![Value::Int16(13), Value::Int16(23), Value::Int16(33)],
        },
        xdf::Sample {
            timestamp: Some(5.4),
            values: vec![Value::Int16(14), Value::Int16(24), Value::Int16(34)],
        },
        xdf::Sample {
            timestamp: Some(5.5),
            values: vec![Value::Int16(15), Value::Int16(25), Value::Int16(35)],
        },
        xdf::Sample {
            timestamp: Some(5.6),
            values: vec![Value::Int16(12), Value::Int16(22), Value::Int16(32)],
        },
        xdf::Sample {
            timestamp: Some(5.7),
            values: vec![Value::Int16(13), Value::Int16(23), Value::Int16(33)],
        },
        xdf::Sample {
            timestamp: Some(5.8),
            values: vec![Value::Int16(14), Value::Int16(24), Value::Int16(34)],
        },
        xdf::Sample {
            timestamp: Some(5.9),
            values: vec![Value::Int16(15), Value::Int16(25), Value::Int16(35)],
        },
    ];

    assert_eq!(first_stream.samples.len(), expected_first_samples.len(), "unexpected number of samples in first stream");
    assert!(match first_stream.format {
        Format::Int16 => true,
        _ => false,
    }, "unexpected format of first stream");

    
    // compare only values
    assert_eq!(expected_first_samples.iter().map(|s| s.values.clone()).collect::<Vec<Vec<Value>>>(), first_stream.samples.iter().map(|s| s.values.clone()).collect::<Vec<Vec<Value>>>(), "first stream values are not as expected");
    
    //then everything
    assert_eq!(expected_first_samples, first_stream.samples, "first stream samples (values and/or timestamps) are not as expected");

    // TODO test second stream

}
