use crate::parse::{Stream};
use winnow::{
    ascii::{line_ending, multispace0},
    combinator::delimited,
    error::{StrContext, StrContextValue},
    prelude::*,
    token::take_while,
};

pub fn newline<'s>(str: &mut Stream) -> winnow::ModalResult<()> {
    take_while(0.., (' ', '\t', '\r')).parse_next(str)?;
    line_ending.parse_next(str)?;
    multispace0.parse_next(str)?;
    Ok(())
}

pub fn ws<'a, F, O, E: winnow::error::ParserError<Stream<'a>>>(inner: F) -> impl Parser<Stream<'a>, O, E>
where
    F: Parser<Stream<'a>, O, E>,
{
    let ws = (' ', '\t', '\r');
    delimited(take_while(0.., ws), inner, take_while(0.., ws))
}

pub fn ws_nl<'a, F, O, E: winnow::error::ParserError<Stream<'a>>>(
    inner: F,
) -> impl Parser<Stream<'a>, O, E>
where
    F: Parser<Stream<'a>, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub fn ctx_desc(str: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(str))
}

pub fn ctx_char(char: char) -> StrContext {
    StrContext::Expected(StrContextValue::CharLiteral(char))
}

pub fn ctx_str(str: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::StringLiteral(str))
}
