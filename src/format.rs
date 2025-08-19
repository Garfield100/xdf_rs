use std::fmt::Display;

/// Possible formats for the data in a stream as given in the specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// signed 8-bit integer
    Int8,
    /// signed 16-bit integer
    Int16,
    /// signed 32-bit integer
    Int32,
    /// signed 64-bit integer
    Int64,
    /// 32-bit floating point number
    Float32,
    /// 64-bit floating point number
    Float64,
    /// UTF-8 encoded string, for example for event markers.
    Str,
}

impl Format {
    pub fn value_size(self) -> Option<u8> {
        match self {
            Format::Int8 => Some(1),
            Format::Int16 => Some(2),
            Format::Int32 => Some(4),
            Format::Int64 => Some(8),
            Format::Float32 => Some(4),
            Format::Float64 => Some(8),
            Format::Str => None,
        }
    }
}

impl From<Format> for &str {
    fn from(format: Format) -> Self {
        match format {
            Format::Int8 => "int8",
            Format::Int16 => "int16",
            Format::Int32 => "int32",
            Format::Int64 => "int64",
            Format::Float32 => "float32",
            Format::Float64 => "double64",
            Format::Str => "string",
        }
    }
}

impl From<&Format> for &str {
    fn from(format: &Format) -> Self {
        <&str>::from(*format)
    }
}

impl From<Format> for String {
    fn from(format: Format) -> Self {
        <&str>::from(format).to_string()
    }
}

impl From<&Format> for String {
    fn from(format: &Format) -> Self {
        String::from(*format)
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&str>::from(*self))
    }
}
