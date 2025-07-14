use crate::parse::stream::{ParseError, ParseErrorCtxHolder, ParseResult, Stream};
use ustr::Ustr;

fn can_start_name(char: char) -> bool {
    char.is_alphabetic() || char == '_'
}

fn can_continue_name(char: char) -> bool {
    can_start_name(char) || char.is_numeric()
}

pub fn parse_name(str: &mut Stream) -> ParseResult<Ustr> {
    str.include_ws(|str| {
        let mut name = String::new();

        let first = str.expect_char_is(can_start_name).ctx_expect_desc("name")?;
        name.push(first);

        while let Ok(next) = str.expect_char_is(can_continue_name) {
            name.push(next);
        }

        Ok(Ustr::from(&name))
    })
}

pub fn parse_schema_name(str: &mut Stream) -> ParseResult<Ustr> {
    str.include_ws(|str| {
        str.expect_char('\'')?;
        parse_name(str)
    })
}

pub fn parse_kw(str: &mut Stream, kw: &str) -> ParseResult<()> {
    str.fallible(|str| {
        str.skip_ws();
        let start = str.checkpoint();
        let name = parse_name(str)
            .ctx_clear(start.0)
            .ctx_expect_str(Ustr::from(kw).as_str())?;

        if name != kw {
            Err(ParseError::new_backtrack(start.0)).ctx_expect_str(Ustr::from(kw).as_str())
        } else {
            Ok(())
        }
    })
}

pub fn parse_num(str: &mut Stream) -> ParseResult<u32> {
    str.include_ws(|str| {
        let first = str
            .expect_char_is(|c| c.is_ascii_digit())
            .ctx_expect_desc("number")?;

        let mut val = first.to_digit(10).unwrap();

        while let Ok(next) = str.expect_char_is(|c| c.is_ascii_digit()) {
            val = val * 10 + next.to_digit(10).unwrap();
        }

        Ok(val)
    })
}
