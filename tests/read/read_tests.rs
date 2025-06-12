use std::fs;

use xdf::{Format, Values, XDFFile};

const EPSILON: f64 = 1E-15;

#[allow(clippy::too_many_lines)]
#[test]
fn read_minimal_xdf() {
    let file_path = "tests/read/minimal.xdf";
    let bytes = fs::read(file_path).unwrap();
    let xdf_file = XDFFile::from_bytes(&bytes).unwrap();

    //must be sorted
    let stream_ids: [u32; 2] = [0, 0x02C0_FFEE];

    assert_eq!(xdf_file.header.name, "info");

    assert_eq!(xdf_file.streams.len(), stream_ids.len());
    let mut read_ids = xdf_file.streams.iter().map(|stream| stream.id).collect::<Vec<u32>>();
    let mut expected_ids = stream_ids;
    read_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(read_ids, expected_ids);

    let first_stream = xdf_file.streams.iter().find(|s| s.id == expected_ids[0]).unwrap();
    let second_stream = xdf_file.streams.iter().find(|s| s.id == expected_ids[1]).unwrap();

    // test first stream
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

    // check length
    assert_eq!(
        expected_first_samples.len(),
        first_stream.samples.len(),
        "unexpected number of samples in first stream. Expected {}, got {}",
        expected_first_samples.len(),
        first_stream.samples.len()
    );

    // check format
    match first_stream.format {
        Format::Int16 => (),
        _ => panic!(
            "unexpected format of first stream. Expected {:?}, got {:?}",
            Format::Int16,
            first_stream.format
        ),
    }

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

    //then the timestamps. compare the reconstructed timestamps using an epsilon
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

    // test second stream
    let footer_string = "<?xml version=\"1.0\"?>
    <info>
        <writer>LabRecorder xdfwriter</writer>
        <first_timestamp>5.1</first_timestamp>
        <last_timestamp>5.9</last_timestamp>
        <sample_count>9</sample_count>
        <clock_offsets>
            <offset>
                <time>50979.76</time>
                <value>-.01</value>
            </offset>
            <offset>
                <time>50979.86</time>
                <value>-.02</value>
            </offset>
        </clock_offsets>
    </info>";

    let expected_second_samples = [
        footer_string,
        "Hello",
        "World",
        "from",
        "LSL",
        "Hello",
        "World",
        "from",
        "LSL",
    ];

    // check length
    assert_eq!(
        second_stream.samples.len(),
        expected_second_samples.len(),
        "unexpected number of samples in second stream. Expected {}, got {}",
        expected_second_samples.len(),
        second_stream.samples.len()
    );

    // check strings
    for (&expected, actual_sample) in expected_second_samples.iter().zip(second_stream.samples.iter()) {
        match actual_sample.values.clone() {
            Values::Strings(strings) => {
                // remove all whitespace because we're comparing XML and I'm too lazy to parse it.
                let mut actual_string = strings;
                actual_string[0].retain(|c| !c.is_whitespace()); // [0] only because we know minimal.xdf only has a single string value here

                let mut expected = vec![expected.to_string()];
                expected[0].retain(|c| !c.is_whitespace()); // [0] only because we know minimal.xdf only has a single string value here

                dbg!(&actual_string);
                dbg!(&expected);

                assert_eq!(
                    actual_string, expected,
                    "Unexpected value in second stream. Expected \n{expected:?}\n, got \n{actual_string:?}\n"
                );
            }
            _ => panic!(
                "Unexpected type in second stream. Expected String, got {:?}",
                actual_sample.values
            ),
        }
    }

    // check format
    match second_stream.format {
        Format::String => (),
        _ => panic!(
            "unexpected format of second stream. Expected {:?}, got {:?}",
            Format::String,
            second_stream.format
        ),
    }
}

#[test]
fn fail_on_invalid_xdf() {
    let invalid_bytes = "This is not a valid XDF file!".as_bytes();
    let xdf_file = XDFFile::from_bytes(invalid_bytes);
    assert!(xdf_file.is_err());
}

#[test]
fn out_of_order_clock_offsets() {
    // this was discovered by fuzzing
    let file_path = "tests/read/out_of_order_clock_offsets.xdf";
    let bytes = fs::read(file_path).unwrap();

    let _xdf_file = XDFFile::from_bytes(&bytes);
}

#[test]
fn chunk_length_lie() {
    // this was discovered by fuzzing
    let file_path = "tests/read/chunk_length_lie.xdf";
    let bytes = fs::read(file_path).unwrap();

    let _xdf_file = XDFFile::from_bytes(&bytes);
}

#[test]
// If I've understood correctly this contains a chunk which is
fn out_of_order_sample_chunk() {
    // this was discovered by fuzzing
    let file_path = "tests/read/out_of_order_sample_chunk.xdf";
    let bytes = fs::read(file_path).unwrap();

    let _xdf_file = XDFFile::from_bytes(&bytes);
}
