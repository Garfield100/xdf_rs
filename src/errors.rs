//! Errors that can occur when parsing a chunk
use std::sync::Arc;


use thiserror::Error;

// #[derive(Debug, Error)]
// pub enum ParseChunkError {
//     /// another general error for when an invalid byte is encountered, such as an invalid number of length bytes being specified.
//     #[error("Invalid chunk bytes. Reason: {msg:?}\nchunk tag: {raw_chunk_tag:?}\nchunk bytes: {raw_chunk_bytes:#?}\nat content byte offset: {offset:?}")]
//     InvalidChunkBytesError {
//         /// the reason for the error
//         msg: String,
//         /// the bytes of the raw chunk that caused the error
//         raw_chunk_bytes: Vec<u8>,
//         /// the tag of the raw chunk that caused the error
//         raw_chunk_tag: u16,
//         /// the offset in the raw chunk's content where the error occurred
//         offset: usize,
//     },
// }

#[derive(Debug, Error)]
pub enum XDFError {
    #[error("The XML element either does not exist or contains invalid or no data: {0}")]
    BadXMLElementError(String),

    #[error("Version {0} is not supported")]
    VersionNotSupportedError(f32),

    #[error("Error parsing chunk")]
    ParseChunkError,

    #[error("Error reading chunk")]
    ReadChunkError,

    #[error("Could not find file header chunk")]
    MissingFileHeaderError,

    #[error("Multiple file header chunks found")]
    MultipleFileHeaderError,

    #[error("Could not find stream header chunk for stream id {0}")]
    MissingStreamHeaderError(u32),

    #[error("Could not find stream footer chunk for stream id {0}")]
    MissingStreamFooterChunk(u32),

    #[error("File does not begin with magic number")]
    NoMagicNumberError,

    #[error("Invalid tag: {0}")]
    InvalidTagError(u16),

    #[error("Invalid number of count bytes. Expected 1, 4, or 8, but got {0}")]
    InvalidNumCountBytes(u8),

    #[error(transparent)]
    TryFromSliceError(#[from] std::array::TryFromSliceError),

    #[error(transparent)]
    XMLParseError(#[from] xmltree::ParseError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error(transparent)]
    NomErr(#[from] nom::Err<nom::error::Error<Arc<[u8]>>>),
}