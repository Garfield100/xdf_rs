//! Errors that can occur when parsing a chunk
use error_chain::error_chain;

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

error_chain! {
    foreign_links {
        TryFromSliceError(std::array::TryFromSliceError);
        XMLParseError(xmltree::ParseError);
        Utf8Error(std::str::Utf8Error);
        IOError(std::io::Error);
        ParseFloatError(std::num::ParseFloatError);
    }

    errors {
        /// relatively general error for when an XML element contains invalid data (e.g. a string when a float is expected) or is missing entirely.
        BadXMLElementError (tag: String) {
            description("The XML element either does not exist or contains invalid or no data")
            display("The XML tag {0} either does not exist or contains invalid or no data", tag)
        }

        /// the version of the XDF file is not supported. Currently, only version 1.0 is supported.
        VersionNotSupportedError(version: f32) {
            description("Version not supported")
            display("Version {} is not supported", version)
        }

        ParseChunkError{
            description("Error parsing chunk")
            display("Error parsing chunk")
        }

        ReadChunkError{
            description("Error reading chunk")
            display("Error reading chunk")
        }

        MissingFileHeaderError {
            description("Could not find file header chunk")
            display("Could not find file header chunk")
        }

        MissingStreamHeaderError(stream_id: u32) {
            description("Could not find stream header chunk")
            display("Could not find stream header chunk for stream id {}", stream_id)
        }

        MissingStreamFooterChunk(stream_id: u32) {
            description("Could not find stream footer chunk")
            display("Could not find stream footer chunk for stream id {}", stream_id)
        }

        NoMagicNumberError {
            description("File does not begin with the bytes 'XDF:'")
            display("File does not begin with magic number")
        }

        InvalidTagError(tag: u16) {
            description("Invalid tag")
            display("Invalid tag: {}", tag)
        }

        InvalidNumCountBytes(num_count_bytes: u8) {
            description("Invalid number of count bytes. Should be 1, 4, or 8")
            display("Invalid number of count bytes. Expected 1, 4, or 8, but got {}", num_count_bytes)
        }

    }
}
