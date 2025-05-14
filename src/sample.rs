use super::Values;

/// A single sample in a stream. Samples may have a timestamp and one or more values.
#[derive(Debug, PartialEq, Clone)]
pub struct Sample {
    /**
    The timestamp of the sample.
    This is optional and may be None if the stream has an irregular sampling rate, as is often the case for marker streams.

    It is worth mentioning that
    * clock offsets are already applied to the timestamps, should they exist
    * most of the timestamps are not actually in the recording but rather calulated using the provided nominal sampling rate.

    Internally, streams are recorded in "chunks". The first sample in a chunk generally includes a timestamp while the rest are calculated.
    */
    pub timestamp: Option<f64>,

    /// The values of the sample.
    pub values: Values,
}

impl PartialOrd for Sample {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

#[test]
fn test_sample_partialord() {
    let sample1 = Sample {
        timestamp: Some(1.0),
        values: Values::Int8(vec![1, 2, 3]),
    };
    let sample2 = Sample {
        timestamp: Some(2.0),
        values: Values::Int8(vec![4, 5, 6]),
    };
    let sample3 = Sample {
        timestamp: Some(3.0),
        values: Values::Int8(vec![7, 8, 9]),
    };

    assert!(sample1 < sample2);
    assert!(sample2 < sample3);
    assert!(sample1 < sample3);
}
