use nom::{number::complete::le_u32, IResult};

pub(super) fn stream_id(input: &[u8]) -> IResult<&[u8], u32> {
    le_u32(input)
}
