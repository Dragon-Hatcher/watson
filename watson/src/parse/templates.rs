use crate::parse::{
    common::parse_name,
    stream::{ParseError, ParseResult, Stream},
};
use ustr::Ustr;

pub enum Template {
    Variable { fresh: bool, name: Ustr },
    Schema { args: u32, name: Ustr },
}

pub fn parse_templates(str: &mut Stream) -> ParseResult<Vec<Template>> {
    let mut templates = Vec::new();

    loop {
        match parse_template(str, &mut templates) {
            Ok(_) => {}
            Err(ParseError::Backtrack) => break,
            Err(ParseError::Commit) => return Err(ParseError::Commit),
        }
    }

    Ok(templates)
}

fn parse_template(str: &mut Stream, templates: &mut Vec<Template>) -> ParseResult<()> {
    str.expect_char('[')?;

    str.commit(move |str| {
        if str.expect_str("schema").is_ok() {
            parse_schemas(str, templates)?;
        } else if str.expect_str("fresh").is_ok() {
            let name = parse_name(str)?;
            templates.push(Template::Variable { fresh: true, name });
        } else {
            let name = parse_name(str)?;
            templates.push(Template::Variable { fresh: false, name });
        }

        str.expect_char(']')?;

        Ok(())
    })
}

fn parse_schemas(str: &mut Stream, templates: &mut Vec<Template>) -> ParseResult<()> {
    let first = str.commit(parse_schema)?;
    templates.push(first);

    loop {
        match parse_schema(str) {
            Ok(s) => templates.push(s),
            Err(ParseError::Backtrack) => break,
            Err(ParseError::Commit) => return Err(ParseError::Commit),
        }
    }

    Ok(())
}

fn parse_schema(str: &mut Stream) -> ParseResult<Template> {
    let name = parse_name(str)?;
    let args = parse_schema_args(str)?;

    Ok(Template::Schema { args, name })
}

fn parse_schema_args(str: &mut Stream) -> ParseResult<u32> {
    if str.expect_char('(').is_err() {
        return Ok(0);
    }

    str.commit(|str| {
        let mut count = 0;

        while str.expect_char('_').is_ok() {
            count += 1;
        }

        str.expect_char(')');
        Ok(count)
    })
}
