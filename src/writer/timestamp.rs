pub struct HasTimestamps;
pub struct NoTimestamps;

pub enum TimestampEnum {
    HasTimestamps,
    NoTimestamps,
}

pub trait Timestamped {
    fn get_enum(&self) -> TimestampEnum;
}

impl Timestamped for HasTimestamps {
    fn get_enum(&self) -> TimestampEnum {
        TimestampEnum::HasTimestamps
    }
}
impl Timestamped for NoTimestamps {
    fn get_enum(&self) -> TimestampEnum {
        TimestampEnum::NoTimestamps
    }
}
