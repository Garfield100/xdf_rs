pub struct HasTimestamps;
pub struct NoTimestamps;

pub enum TimestampEnum {
    HasTimestamps,
    NoTimestamps,
}

pub trait Timestamped {
    fn get_enum() -> TimestampEnum;
    fn is_timestamped() -> bool {
        matches!(Self::get_enum(), TimestampEnum::HasTimestamps)
    }
}

impl Timestamped for HasTimestamps {
    fn get_enum() -> TimestampEnum {
        TimestampEnum::HasTimestamps
    }
}
impl Timestamped for NoTimestamps {
    fn get_enum() -> TimestampEnum {
        TimestampEnum::NoTimestamps
    }
}
