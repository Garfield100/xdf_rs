use crate::writer::Sealed;

/// Marker type used to specify that a stream should have timestamps using the generic parameters.
#[derive(Debug, Clone, Copy)]
pub struct HasTimestamps;
/// Marker type used to specify that a stream should not have timestamps using the generic parameters.
#[derive(Debug, Clone, Copy)]
pub struct NoTimestamps;

#[allow(private_bounds)] // the point is to make this trait visible but not implementable
pub trait TimestampTrait: Sealed {}

impl Sealed for HasTimestamps {}
impl Sealed for NoTimestamps {}
impl TimestampTrait for HasTimestamps {}
impl TimestampTrait for NoTimestamps {}