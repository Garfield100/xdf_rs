use nom::{combinator, error::context, IResult};
use xmltree::Element;

// bit of an odd one but so be it
pub(crate) fn xml(input: &[u8]) -> IResult<&[u8], Element> {
    let Ok(xml) = Element::parse(input) else {
        return context("xml error parsing xml", combinator::fail)(input);
    };

    Ok((input, xml))
}
