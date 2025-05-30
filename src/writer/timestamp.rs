pub struct HasTimestamps;
pub struct NoTimestamps;

pub enum TimestampEnum {
    HasTimestamps,
    NoTimestamps,
}

pub trait TimestampTrait {
    fn get_enum() -> TimestampEnum;
    fn is_timestamped() -> bool {
        matches!(Self::get_enum(), TimestampEnum::HasTimestamps)
    }
}

impl TimestampTrait for HasTimestamps {
    fn get_enum() -> TimestampEnum {
        TimestampEnum::HasTimestamps
    }
}
impl TimestampTrait for NoTimestamps {
    fn get_enum() -> TimestampEnum {
        TimestampEnum::NoTimestamps
    }
}
