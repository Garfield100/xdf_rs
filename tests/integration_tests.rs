use std::{fs, path::PathBuf};

use xdf::{Format, Values, XDFFile};

const EPSILON: f64 = 1E-15;

#[test]
fn read_minimal_xdf() {
    let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file_path.push("tests/minimal.xdf");
    let bytes = fs::read(file_path).unwrap();
    let xdf_file = XDFFile::from_bytes(&bytes).unwrap();

    //must be sorted
    let stream_ids: [u32; 2] = [0, 0x02C0FFEE];

    assert_eq!(xdf_file.header.name, "info");

    assert_eq!(xdf_file.streams.len(), stream_ids.len());
    let mut read_ids = xdf_file.streams.iter().map(|stream| stream.id).collect::<Vec<u32>>();
    let mut expected_ids = stream_ids.clone();
    read_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(read_ids.as_slice(), stream_ids);

    let first_stream = xdf_file.streams.iter().find(|s| s.id == expected_ids[0]).unwrap();
    let _second_stream = xdf_file.streams.iter().find(|s| s.id == expected_ids[1]).unwrap();

    // timestamps minus the clock offsets (always -0.1 in this file)
    let expected_first_samples = vec![
        xdf::Sample {
            timestamp: Some(5.1 - 0.1),
            values: Values::Int16(vec![192, 255, 238]),
        },
        xdf::Sample {
            timestamp: Some(5.2 - 0.1),
            values: Values::Int16(vec![12, 22, 32]),
        },
        xdf::Sample {
            timestamp: Some(5.3 - 0.1),
            values: Values::Int16(vec![13, 23, 33]),
        },
        xdf::Sample {
            timestamp: Some(5.4 - 0.1),
            values: Values::Int16(vec![14, 24, 34]),
        },
        xdf::Sample {
            timestamp: Some(5.5 - 0.1),
            values: Values::Int16(vec![15, 25, 35]),
        },
        xdf::Sample {
            timestamp: Some(5.6 - 0.1),
            values: Values::Int16(vec![12, 22, 32]),
        },
        xdf::Sample {
            timestamp: Some(5.7 - 0.1),
            values: Values::Int16(vec![13, 23, 33]),
        },
        xdf::Sample {
            timestamp: Some(5.8 - 0.1),
            values: Values::Int16(vec![14, 24, 34]),
        },
        xdf::Sample {
            timestamp: Some(5.9 - 0.1),
            values: Values::Int16(vec![15, 25, 35]),
        },
    ];

    assert_eq!(
        first_stream.samples.len(),
        expected_first_samples.len(),
        "unexpected number of samples in first stream. Expected {}, got {}",
        expected_first_samples.len(),
        first_stream.samples.len()
    );
    assert!(
        match first_stream.format {
            Format::Int16 => true,
            _ => false,
        },
        "unexpected format of first stream. Expected {:?}, got {:?}",
        Format::Int16,
        first_stream.format
    );

    // compare only values
    assert_eq!(
        expected_first_samples
            .iter()
            .map(|s| s.values.clone())
            .collect::<Vec<Values>>(),
        first_stream
            .samples
            .iter()
            .map(|s| s.values.clone())
            .collect::<Vec<Values>>(),
        "first stream values are not as expected"
    );

    //then the timestamps. we have to loop because we need an epsilon to compare the reconstructed timestamps using an epsilon
    for (i, (actual_sample, expected_sample)) in
        Iterator::zip(first_stream.samples.iter(), expected_first_samples.iter()).enumerate()
    {
        assert!(
            actual_sample.timestamp.is_some(),
            "timestamp of sample {} in first stream is None, expected {:?}",
            i,
            expected_sample.timestamp
        );
        assert!(
            (actual_sample.timestamp.unwrap() - expected_sample.timestamp.unwrap()).abs() < EPSILON,
            "timestamp of sample {} in first stream is {}, expected {} to be within {} of it",
            i,
            actual_sample.timestamp.unwrap(),
            expected_sample.timestamp.unwrap(),
            EPSILON
        );
    }

    // TODO test second stream
}
