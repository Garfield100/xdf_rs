use std::{fs, path::PathBuf};

use xdf::{Format, Values, XDFFile};

const EPSILON: f64 = 0.000000000000001;

#[test]
fn read_minimal_xdf() {
    let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file_path.push("example-files/minimal.xdf");
    let bytes = fs::read(file_path).unwrap();
    let xdf_file = XDFFile::from_bytes(&bytes).unwrap();

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
            values: Values::Int16(vec![192, 255, 238]),
        },
        xdf::Sample {
            timestamp: Some(5.2),
            values: Values::Int16(vec![12, 22, 32]),
        },
        xdf::Sample {
            timestamp: Some(5.3),
            values: Values::Int16(vec![13, 23, 33]),
        },
        xdf::Sample {
            timestamp: Some(5.4),
            values: Values::Int16(vec![14, 24, 34]),
        },
        xdf::Sample {
            timestamp: Some(5.5),
            values: Values::Int16(vec![15, 25, 35]),
        },
        xdf::Sample {
            timestamp: Some(5.6),
            values: Values::Int16(vec![12, 22, 32]),
        },
        xdf::Sample {
            timestamp: Some(5.7),
            values: Values::Int16(vec![13, 23, 33]),
        },
        xdf::Sample {
            timestamp: Some(5.8),
            values: Values::Int16(vec![14, 24, 34]),
        },
        xdf::Sample {
            timestamp: Some(5.9),
            values: Values::Int16(vec![15, 25, 35]),
        },
    ];

    assert_eq!(
        first_stream.samples.len(),
        expected_first_samples.len(),
        "unexpected number of samples in first stream"
    );
    assert!(
        match first_stream.format {
            Format::Int16 => true,
            _ => false,
        },
        "unexpected format of first stream"
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
            "timestamp of sample {} in first stream was {}, expected {} to be within {} of it",
            i,
            actual_sample.timestamp.unwrap(),
            expected_sample.timestamp.unwrap(),
            EPSILON
        );
    }

    // TODO test second stream
}
