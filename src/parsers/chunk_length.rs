use nom::{combinator::value, bytes::complete::tag, number::complete::{le_u8, le_u32, le_u64}, IResult, Parser};

// num length bytes parser
fn num_length_bytes(input: &[u8]) -> IResult<&[u8], u8> {
    let parse_1 = value(1, tag([1_u8]));
    let parse_4 = value(4, tag([4_u8]));
    let parse_8 = value(8, tag([8_u8]));

    nom::branch::alt((parse_1, parse_4, parse_8)).parse(input)
}

// length parser
pub(crate) fn length(input: &[u8]) -> IResult<&[u8], usize> {
    let (input, num_length_bytes) = num_length_bytes(input)?;

    match num_length_bytes {
        1 => {
            let (input, length) = le_u8(input)?;
            Ok((input, length as usize))
        }
        4 => {
            let (input, length) = le_u32(input)?;
            Ok((input, length as usize))
        }
        8 => {
            let (input, length) = le_u64(input)?;
            Ok((input, length as usize))
        }
        _ => Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::LengthValue,
        ))),
    }
}