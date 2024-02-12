use nom::{error::context, multi, number, IResult};

use crate::{Format, Values};

use super::chunk_length::length;

// string value structure
// [NumLengthBytes] [Length] [StringContent]
// [1, 4, or 8] [...] [Arbitrary]
// [1] [As encoded] [Length]

fn string_value(input: &[u8]) -> IResult<&[u8], String> {
    let (input, length) = length(input)?;
    let (input, string) = nom::bytes::complete::take(length)(input)?;
    let string = String::from_utf8(string.to_vec()).unwrap();

    Ok((input, string))
}

// structure of a value:
// [double, float, int64, int32, int16 or int8]
// [Arbitrary]
// [8, 4, 2 or 1]

pub(super) fn values(input: &[u8], format: Format, num_values: usize) -> IResult<&[u8], Values> {
    let mut input = input;
    let values = match format {
        Format::Float32 => {
            let (inp, values) = context("values Float32", multi::count(number::complete::le_f32, num_values))(input)?;
            input = inp;
            Values::Float32(values)
        }
        Format::Float64 => {
            let (inp, values) = context("values Float64", multi::count(number::complete::le_f64, num_values))(input)?;
            input = inp;
            Values::Float64(values)
        }
        Format::Int8 => {
            let (inp, values) = context("values Int8", multi::count(number::complete::le_i8, num_values))(input)?;
            input = inp;
            Values::Int8(values)
        }
        Format::Int16 => {
            let (inp, values) = context("values Int16", multi::count(number::complete::le_i16, num_values))(input)?;
            input = inp;
            Values::Int16(values)
        }
        Format::Int32 => {
            let (inp, values) = context("values Int32", multi::count(number::complete::le_i32, num_values))(input)?;
            input = inp;
            Values::Int32(values)
        }
        Format::Int64 => {
            let (inp, values) = context("values Int64", multi::count(number::complete::le_i64, num_values))(input)?;
            input = inp;
            Values::Int64(values)
        }
        Format::String => {
            let (inp, string) = context("values String", string_value)(input)?;
            input = inp;
            Values::String(string)
        }
    };

    Ok((input, values))
}
