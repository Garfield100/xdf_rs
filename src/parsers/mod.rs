mod chunk_length;
mod chunk_tags;

mod boundary;
mod clock_offset;
mod file_header;
mod samples;
mod stream_footer;
mod stream_header;
mod stream_id;
mod values;
mod xml;

pub(crate) mod xdf_file;

use boundary::boundary;
use clock_offset::clock_offset;
use file_header::file_header;
use samples::samples;
use stream_footer::stream_footer;
use stream_header::stream_header;
use stream_id::stream_id;
use values::values;
use xml::xml;
