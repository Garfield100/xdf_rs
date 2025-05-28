use std::{
    io::{self},
    sync::PoisonError,
};

#[derive(thiserror::Error, Debug)]
pub enum XDFWriterError {
    #[error(transparent)]
    XMLTree(#[from] xmltree::Error),
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Conversion(#[from] std::num::TryFromIntError),
    #[error("XDFWriter Mutex is poisoned")]
    PoisonError,
    #[error("Expected {expected} values for {expected} channels but got {actual} values")]
    LengthMismatch { expected: usize, actual: usize },
}

// TODO restrict this immpl somewhat
impl<T> From<PoisonError<T>> for XDFWriterError {
    fn from(_value: PoisonError<T>) -> Self {
        XDFWriterError::PoisonError
    }
}
