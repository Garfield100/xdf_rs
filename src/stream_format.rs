use std::fmt::Debug;

use nom::{combinator, error::context, IResult};
use tracing::trace;
use zerocopy::{transmute_ref, FromBytes, Immutable, IntoBytes};

use crate::{errors::ParseError, parsers::chunk_length::length, writer::Sealed, Format};

macro_rules! define_stream_type {
    ($name:ty, $format:expr) => {
        impl Sealed for $name {}
        impl StreamFormat for $name {
            fn format() -> Format {
                $format
            }
        }
    };
}

// TODO is this used anywhere? lmao
pub(crate) trait MyFromBytes: Sized {
    fn from_bytes(input: &[u8]) -> Result<Vec<Self>, ParseError>;
}

impl<T: NumberFormat> MyFromBytes for T {
    /// creates a Vec<T> from a byte slice for the number types, i.e. integers and floats
    fn from_bytes(input: &[u8]) -> Result<Vec<Self>, ParseError> {
        // make sure the number of bytes we get is a multiple of the size of T
        if input.len() % size_of::<T>() == 0 {
            let nums: &[Self] = transmute_ref!(input);
            Ok(nums.to_vec())
        } else {
            Err(ParseError::Values(T::format()))
        }
    }
}

fn string_value(input: &[u8]) -> IResult<&[u8], String> {
    let (input, length) = length(input)?;
    trace!("String value is {length} bytes long");

    let (input, string_bytes) = nom::bytes::complete::take(length)(input)?;
    let Ok(string) = String::from_utf8(string_bytes.to_vec()) else {
        return context("string_value invalid utf8", combinator::fail)(&[0]);
    };

    Ok((input, string))
}

impl MyFromBytes for String {
    fn from_bytes(input: &[u8]) -> Result<Vec<Self>, ParseError> {
        let (input, strings) =
            nom::multi::many0(string_value)(input).map_err(|_| ParseError::Values(Format::String))?;

        if input.is_empty() {
            Err(ParseError::Values(Format::String))
        } else {
            Ok(strings.into_iter().collect())
        }
    }
}

/// Trait implemented for valid stream formats.
///
/// Mostly a marker trait. Sealed as it is not meant to be implemented for other types.
#[allow(private_bounds)]
pub trait StreamFormat: Sized + Debug + Sealed + Clone + PartialEq + MyFromBytes {
    /// Returns the [`Format`] associated with this type
    fn format() -> Format;
}

struct ImmutableString(String);

define_stream_type!(i8, Format::Int8);
define_stream_type!(i16, Format::Int16);
define_stream_type!(i32, Format::Int32);
define_stream_type!(i64, Format::Int64);
define_stream_type!(f32, Format::Float32);
define_stream_type!(f64, Format::Float64);
define_stream_type!(String, Format::String);

/// Marker trait for stream formats which are not string.
///
/// Sealed as it is not meant to be implemented for other types.
#[allow(private_bounds)]
pub trait NumberFormat: StreamFormat + IntoBytes + FromBytes + Immutable + Sealed {}
impl NumberFormat for i8 {}
impl NumberFormat for i16 {}
impl NumberFormat for i32 {}
impl NumberFormat for i64 {}
impl NumberFormat for f32 {}
impl NumberFormat for f64 {}
