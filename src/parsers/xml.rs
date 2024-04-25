use nom::{bytes::complete::take, combinator, error::context, IResult, Parser};
use xmltree::Element;

pub(crate) fn xml(input: &[u8], length: usize) -> IResult<&[u8], Element> {
    let (input, content) = take(length).parse(input)?;
    let Ok(xml) = Element::parse(content) else {
        return context("xml error parsing xml", combinator::fail)(&[0]);
    };

    Ok((input, xml))
}
