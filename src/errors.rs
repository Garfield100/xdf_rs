//! Errors that can occur when parsing a chunk


use error_chain::error_chain;
use std::io::{self};
use thiserror::Error;

//TODO rewrite using error_chain only, instead of using both thiserror and error_chain
/// Errors that can occur when parsing a chunk.
#[derive(Debug, Error)]
pub enum ParseChunkError {
    /// passes up an error from the xmltree crate.
    #[error(transparent)]
    XMLParseError(#[from] xmltree::ParseError),

    /// relatively general error for when an XML element contains invalid data (e.g. a string when a float is expected) or is missing entirely.
    #[error("The XML tag {0} either does not exist or contains invalid or no data")]
    BadElementError(String),

    /// the version of the XDF file is not supported. Currently, only version 1.0 is supported.
    #[error("Version {0} is not supported")]
    VersionNotSupportedError(f32),

    /// bytes could not be parsed into a valid utf8 string.
    #[error(transparent)]
    Utf8Error(#[from] io::Error),

    /// another general error for when an invalid byte is encountered, such as an invalid number of length bytes being specified.
    #[error("Invalid chunk bytes. Reason: {msg:?}\nchunk tag: {raw_chunk_tag:?}\nchunk bytes: {raw_chunk_bytes:#?}\nat content byte offset: {offset:?}")]
    InvalidChunkBytesError {
        /// the reason for the error
        msg: String,
        /// the bytes of the raw chunk that caused the error
        raw_chunk_bytes: Vec<u8>,
        /// the tag of the raw chunk that caused the error
        raw_chunk_tag: u16,
        /// the offset in the raw chunk's content where the error occurred
        offset: usize,
    },

    /// Every stream must have a stream header. This error is thrown when a stream header is not found for a streamID which is referenced elsewhere.
    #[error("Could not find stream header chunk for stream id {stream_id:?}. File is either invalid or the chunks are somehow out of order, as file and stream headers must be at the beginning of the file.")]
    MissingHeaderError {
        /// the streamID that was referenced but not found
        stream_id: u32,
    },
    // #[error("Other error. Reason: {0}")]
    // Other(String),
}

/// Errors that can occur when reading a chunk
#[derive(Debug, Error)]
pub enum ReadChunkError {
    //TODO: replace this with more specific errors
    /// general error
    #[error("Could not parse file: {0}")]
    ParseError(String),

    /// This error is thrown when the file ends unexpectedly.
    #[error("File is too short, reached EOF unexpectedly")]
    EOFError,

    /// The first 4 bytes of a valid XDF file must be "XDF:". This error is thrown when this is not the case.
    #[error("File does not begin with magic number")]
    NoMagicNumberError,

    /// In version 1.0 of the specification, only tags 1 through 6 inclusive are valid. This is thrown when a tag outside of that range is encountered.
    #[error("Invalid tag. Expected 1 to 6 inclusive but was {0}")]
    InvalidTagError(u16),

    /// general IO error passed up from std::io.
    #[error(transparent)]
    IOError(#[from] io::Error),
}

error_chain! {
    foreign_links {
        ParseChunkError(ParseChunkError);
        ReadChunkError(ReadChunkError);
    }

    errors {
        MissingFileHeaderChunk {
            description("Could not find file header chunk")
            display("Could not find file header chunk")
        }

        MissingStreamHeaderChunk(stream_id: u32) {
            description("Could not find stream header chunk")
            display("Could not find stream header chunk for stream id {}", stream_id)
        }

        MissingStreamFooterChunk(stream_id: u32) {
            description("Could not find stream footer chunk")
            display("Could not find stream footer chunk for stream id {}", stream_id)
        }


    }
}
