// let stream = StreamWriter {b
//             state: self.state.clone(),
//             info: stream_info,
//             id: stream_id,
//             _timestamp_marker: PhantomData::<T>,
//             _format_marker: PhantomData::<F>,
//         };

//         let writer = &mut self.state.lock()?.writer;

use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use strict_num::PositiveF64;

use super::{
    error::XDFWriterError,
    stream_format::StreamFormat,
    timestamp::{HasTimestamps, NoTimestamps, Timestamped},
    SharedState, StreamID, StreamInfo,
};

pub struct StreamWriter<W: Write, F: StreamFormat, T: Timestamped> {
    pub(crate) state: Arc<Mutex<SharedState<W>>>,
    pub(crate) info: StreamInfo,
    pub(crate) id: StreamID,
    pub(crate) _timestamp_marker: std::marker::PhantomData<T>,
    pub(crate) _format_marker: std::marker::PhantomData<F>,
}

impl<W: Write, F: StreamFormat> StreamWriter<W, F, HasTimestamps> {
    pub fn write_samples(&mut self, samples: &[F], first_timestamp: PositiveF64) -> Result<(), XDFWriterError> {
        let mut state_lock = self.state.lock()?;
        let write_helper = &mut state_lock.write_helper;

        todo!()
    }
}

impl<W: Write, F: StreamFormat> StreamWriter<W, F, NoTimestamps> {
    pub fn write_samples() {}
}
