//! Errors that can occur when parsing a chunk
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XDFError {
    #[error(transparent)]
    Xml(#[from] XMLError),

    #[error(transparent)]
    Stream(#[from] StreamError),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum XMLError {
    #[error("The XML element either does not exist or contains invalid or no data: {0}")]
    BadElement(String),

    #[error(transparent)]
    ParseError(#[from] xmltree::ParseError),
}

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("Could not find stream header chunk for stream id {0}")]
    MissingHeader(u32),

    #[error("Could not find stream footer chunk for stream id {0}")]
    MissingFooter(u32),

    #[error("Could not find file header chunk")]
    MissingFileHeader,

    #[error("Multiple file header chunks found")]
    MultipleFileHeader,

    #[error("Version {0} is not supported")]
    UnsupportedVersion(f32),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Error parsing chunk")]
    ChunkParse,

    #[error("Error reading chunk")]
    ChunkRead,

    #[error("File does not begin with magic number")]
    NoMagicNumber,

    #[error("Invalid tag: {0}")]
    InvalidTag(u16),

    #[error("Invalid number of count bytes. Expected 1, 4, or 8, but got {0}")]
    InvalidNumCountBytes(u8),

    #[error("There is something wrong with the samples")]
    InvalidSample,

    #[error("Encountered an invalid clock offset")]
    InvalidClockOffset,

    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),

    #[error(transparent)]
    Nom(#[from] nom::Err<nom::error::Error<Arc<[u8]>>>),
}