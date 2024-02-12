use nom::{bytes::complete::take, IResult, Parser};
use xmltree::Element;

pub(crate) fn xml(input: &[u8], length: usize) -> IResult<&[u8], Element> {
    let (input, content) = take(length).parse(input)?;
    let xml = Element::parse(content).unwrap();

    Ok((input, xml))
}
