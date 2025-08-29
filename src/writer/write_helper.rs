use std::io::Write;

use super::error::XDFWriterError;
use strict_num::PositiveF64;
use xmltree::Element;

use crate::{chunk_structs::Tag, writer::length_bytes, StreamID};


#[derive(Debug)]
pub(crate) struct WriteHelper<W: Write> {
    pub(crate) writer: W,
}

impl<W: Write> WriteHelper<W> {
    fn write_magic_num(&mut self) -> Result<(), std::io::Error> {
        const MAGIC_NUM: &[u8; 4] = b"XDF:";
        self.writer.write_all(MAGIC_NUM)?;
        Ok(())
    }

    pub(crate) fn write_file_header(&mut self, xml: &Element) -> Result<(), XDFWriterError> {
        self.write_magic_num()?;

        let mut xml_bytes = Vec::new();
        xml.write(&mut xml_bytes)?;

        self.write_chunk(Tag::FileHeader, &xml_bytes)?;

        Ok(())
    }

    pub(crate) fn write_stream_header(&mut self, id: StreamID, xml: &Element) -> Result<(), XDFWriterError> {
        let id_bytes = id.to_le_bytes();

        let mut bytes = Vec::from(id_bytes);

        xml.write(&mut bytes)?;

        self.write_chunk(Tag::StreamHeader, &bytes)?;

        Ok(())
    }

    pub(crate) fn write_stream_footer(&mut self, id: StreamID, xml: &Element) -> Result<(), XDFWriterError> {
        let id_bytes = id.to_le_bytes();
        let mut bytes = Vec::from(id_bytes);

        xml.write(&mut bytes)?;

        self.write_chunk(Tag::StreamFooter, &bytes)?;

        Ok(())
    }

    pub(crate) fn write_boundary(&mut self) -> Result<(), XDFWriterError> {
        const BOUNDARY_BYTES: [u8; 16] = [
            0x43, 0xA5, 0x46, 0xDC, 0xCB, 0xF5, 0x41, 0x0F, 0xB3, 0x0E, 0xD5, 0x46, 0x73, 0x83, 0xCB, 0xE4,
        ];

        self.write_chunk(Tag::Boundary, &BOUNDARY_BYTES)?;

        Ok(())
    }

    pub(crate) fn write_clock_offset(
        &mut self,
        id: StreamID,
        collection_time: PositiveF64,
        offset_value: PositiveF64,
    ) -> Result<(), XDFWriterError> {
        let id_bytes = id.to_le_bytes();
        let ct_bytes = collection_time.get().to_le_bytes();
        let off_bytes = offset_value.get().to_le_bytes();

        // one day we will be able to do this in place without unsafe qwq. Is optimised out anyway.
        let mut bytes: [u8; 20] = [0; 20];
        bytes[0..4].copy_from_slice(&id_bytes);
        bytes[4..12].copy_from_slice(&ct_bytes);
        bytes[12..20].copy_from_slice(&off_bytes);

        self.write_chunk(Tag::ClockOffset, &bytes)?;

        Ok(())
    }
    pub(crate) const fn get_writer(&mut self) -> &mut W {
        &mut self.writer
    }

    fn write_chunk(&mut self, chunk_tag: Tag, chunk_bytes: &[u8]) -> Result<(), std::io::Error> {
        // 1 num length byte, max. 8 length bytes, 2 Tag bytes, and chunk bytes
        // let mut bytes = Vec::with_capacity(1 + 8 + 2 + chunk_bytes.len());

        // two tag bytes which specify what kind of chunk it is
        let tag_bytes: [u8; 2] = chunk_tag.as_bytes();

        // one byte specifying the number of length bytes, and then N bytes containing the actual length, including the tag
        self.writer
            .write_all(length_bytes!(chunk_bytes.len() + tag_bytes.len()))?;

        // bytes.extend_from_slice(&tag_bytes);
        self.writer.write_all(&tag_bytes)?;

        // the chunk's actual byte content
        // bytes.extend_from_slice(chunk_bytes);
        self.writer.write_all(chunk_bytes)?;

        Ok(())
    }
}

#[test]
fn test_magic_num() {
    let mut buf: [u8; 4] = [0; 4];
    let mut write_helper = WriteHelper {
        writer: buf.as_mut_slice(),
    };

    write_helper.write_magic_num().expect("Error while writing magic num");

    assert_eq!(&buf, b"XDF:");
}
