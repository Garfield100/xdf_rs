use nom::{
    bytes::complete::tag,
    combinator::value,
    number::complete::{le_u32, le_u64, le_u8},
    IResult, Parser,
};

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

#[test]
fn test_num_length_bytes() {
    let valid_inputs = [1, 4, 8];
    for input in valid_inputs.iter() {
        let input_slice = [*input];
        let result = num_length_bytes(&input_slice);
        assert!(result.is_ok());
        let (remainder, len_bytes) = result.unwrap();
        assert!(remainder.is_empty()); // no remaining bytes
        assert_eq!(len_bytes, *input);
    }

    let invalid_inputs = [0, 2, 3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15];
    for invalid_input in invalid_inputs.iter() {
        let input = [*invalid_input];
        let result = num_length_bytes(&input);
        assert!(result.is_err());
    }
}

#[test]
fn test_length() {
    let beef: u32 = 0xBEEF;
    let beef_bytes = beef.to_le_bytes();
    let mut input = vec![4_u8];
    input.extend_from_slice(&beef_bytes);
    let result = length(&input);

    assert!(result.is_ok());
    let (remainder, len) = result.unwrap();
    assert!(remainder.is_empty());
    assert_eq!(len, beef as usize);
}
