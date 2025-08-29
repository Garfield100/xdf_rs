use nom::{bytes::complete::take, combinator, error::context, multi, number, IResult};
use tracing::{instrument, trace};

use crate::{Format};

use super::chunk_length::length;

// string value structure
// [NumLengthBytes] [Length] [StringContent]
// [1, 4, or 8] [...] [Arbitrary]
// [1] [As encoded] [Length]

fn string_value_size(input: &[u8]) -> IResult<&[u8], u64> {
    let (input, length) = length(input)?;
    trace!("String value is {length} bytes long");

    let (input, _string_bytes) = take(length)(input)?;

    Ok((input, length))
}

// structure of a value:
// [double, float, int64, int32, int16 or int8]
// [Arbitrary]
// [8, 4, 2 or 1]
#[instrument(level = "trace", skip(input), ret)]
pub(super) fn values_bytes(input: &[u8], format: Format, num_values: usize) -> IResult<&[u8], &[u8]> {
    let mut input = input;
    let values = match format {
        Format::Float32 => {
            let values_bytes;
            (input, values_bytes) = context("values Float32", take(4 * num_values))(input)?;
            values_bytes
        }
        Format::Float64 => {
            let values_bytes;
            (input, values_bytes) = context("values Float32", take(8 * num_values))(input)?;
            values_bytes
        }
        Format::Int8 => {
            let values_bytes;
            (input, values_bytes) = context("values Float32", take(1 * num_values))(input)?;
            values_bytes
        }
        Format::Int16 => {
            let values_bytes;
            (input, values_bytes) = context("values Float32", take(2 * num_values))(input)?;
            values_bytes
        }
        Format::Int32 => {
            let values_bytes;
            (input, values_bytes) = context("values Float32", take(4 * num_values))(input)?;
            values_bytes
        }
        Format::Int64 => {
            let values_bytes;
            (input, values_bytes) = context("values Float32", take(8 * num_values))(input)?;
            values_bytes
        }
        Format::String => {
            // do not "consume" the input when checking for the total length
            let (_, string_bytes_len) = context(
                "values String len",
                multi::fold_many_m_n(num_values, num_values, string_value_size, || 0_u64, u64::wrapping_add),
            )(input)?;

            let values_bytes;
            (input, values_bytes) = context("values String", take(string_bytes_len))(input)?;
            values_bytes
        }
    };

    Ok((input, values))
}
