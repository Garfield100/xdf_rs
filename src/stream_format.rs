use std::fmt::Debug;

use nom::{combinator, error::context, Finish, IResult};
#[cfg(feature = "tracing")]
use tracing::trace;
use zerocopy::{FromBytes, Immutable, IntoBytes};

use crate::{errors::ParseError, parsers::chunk_length::length, Sealed, Format};

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

macro_rules! impl_my_from_bytes {
    ($name:ty) => {
        impl MyFromBytes for $name {
            /// creates a Vec<T> from a byte slice for the number types, i.e. integers and floats
            fn from_bytes(input: &[u8]) -> Result<Vec<Self>, ParseError> {
                // make sure the number of bytes we get is a multiple of the size of T
                if input.len() % size_of::<Self>() == 0 {
                    let nums: Result<Vec<Self>, _> = input
                        .chunks_exact(size_of::<Self>())
                        .map(|chunk| -> Result<_, ParseError> { Ok(Self::from_le_bytes(chunk.try_into()?)) })
                        .collect();

                    nums
                } else {
                    Err(ParseError::Values(Self::format()))
                }
            }
        }
    };
}

impl_my_from_bytes!(i8);
impl_my_from_bytes!(i16);
impl_my_from_bytes!(i32);
impl_my_from_bytes!(i64);
impl_my_from_bytes!(f32);
impl_my_from_bytes!(f64);


fn string_value(input: &[u8]) -> IResult<&[u8], String> {
    let (input, length) = length(input)?;

    #[cfg(feature = "tracing")]
    trace!("String value is {length} bytes long");

    let (input, string_bytes) = nom::bytes::complete::take(length)(input)?;
    let Ok(string) = String::from_utf8(string_bytes.to_vec()) else {
        return context("string_value invalid utf8", combinator::fail)(&[0]);
    };

    Ok((input, string))
}

impl MyFromBytes for String {
    fn from_bytes(input: &[u8]) -> Result<Vec<Self>, ParseError> {
        let (input, strings) = nom::multi::many0(string_value)(input)
            .finish()
            .map_err(|_| ParseError::Values(Format::String))?;

        if !input.is_empty() {
            Err(ParseError::Values(Format::String))
        } else {
            Ok(strings)
        }
    }
}

/// Trait implemented for valid stream formats.
///
/// Mostly a marker trait. Sealed as it is not meant to be implemented for other types.
#[expect(private_bounds)]
pub trait StreamFormat: Sized + Debug + Sealed + Clone + PartialEq + MyFromBytes {
    /// Returns the [`Format`] associated with this type
    fn format() -> Format;
}

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
#[expect(private_bounds)]
pub trait NumberFormat: StreamFormat + IntoBytes + FromBytes + Immutable + Sealed {}
impl NumberFormat for i8 {}
impl NumberFormat for i16 {}
impl NumberFormat for i32 {}
impl NumberFormat for i64 {}
impl NumberFormat for f32 {}
impl NumberFormat for f64 {}
