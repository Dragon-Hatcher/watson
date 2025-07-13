use crate::parse::stream::{ParseResult, Stream};
use ustr::Ustr;

fn can_start_name(char: char) -> bool {
    char.is_alphabetic() || char == '\'' || char == '_'
}

fn can_continue_name(char: char) -> bool {
    can_start_name(char) || char.is_numeric()
}

pub fn parse_name(str: &mut Stream) -> ParseResult<Ustr> {
    let mut name = String::new();

    let first = str.expect_char_is(can_start_name)?;
    name.push(first);

    while let Ok(next) = str.expect_char_is(can_continue_name) {
        name.push(next);
    }

    Ok(Ustr::from(&name))
}
