use std::fmt::Debug;

use zerocopy::{Immutable, IntoBytes};

use crate::{writer::Sealed, Format};

macro_rules! define_stream_type {
    ($name:ty, $format:expr) => {
        impl Sealed for $name {}
        impl StreamFormat for $name {
            fn get_format() -> Format {
                $format
            }
        }
    };
}

/// Trait implemented for valid stream formats.
///
/// Mostly a marker trait. Sealed as it is not meant to be implemented for other types.
#[allow(private_bounds)]
pub trait StreamFormat: Sized + Debug + Immutable + Sealed {
    /// Returns the [`Format`] associated with this type
    fn get_format() -> Format;
}

define_stream_type!(i8, Format::Int8);
define_stream_type!(i16, Format::Int16);
define_stream_type!(i32, Format::Int32);
define_stream_type!(i64, Format::Int64);
define_stream_type!(f32, Format::Float32);
define_stream_type!(f64, Format::Float64);
define_stream_type!(&str, Format::String);

/// Marker trait for stream formats which are not string.
///
/// Sealed as it is not meant to be implemented for other types.
#[allow(private_bounds)]
pub trait NumberFormat: StreamFormat + IntoBytes + Sealed {}
impl NumberFormat for i8 {}
impl NumberFormat for i16 {}
impl NumberFormat for i32 {}
impl NumberFormat for i64 {}
impl NumberFormat for f32 {}
impl NumberFormat for f64 {}
