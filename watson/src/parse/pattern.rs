use std::sync::atomic::{AtomicUsize, Ordering};

use crate::parse::{
    common::{parse_name, parse_num},
    stream::{ParseError, ParseResult, Stream},
};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternId(usize);

impl PatternId {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::SeqCst);
        Self(id)
    }
}

#[derive(Debug)]
pub struct Precedence(u32);

#[derive(Debug)]
pub struct Pattern {
    pub id: PatternId,
    pub precedence: Precedence,
    pub parts: Vec<(Option<Ustr>, PatternPart)>,
}

#[derive(Debug)]
pub enum PatternPart {
    Sentence,
    Value,
    Binding,
    Lit(Ustr),
}

#[derive(Debug)]
pub enum PatternTy {
    Sentence,
    Value,
}

pub fn parse_pattern(str: &mut Stream) -> ParseResult<Pattern> {
    str.fallible(|str| {
        let precedence = Precedence(parse_num(str)?);
        str.expect_char('|')?;
        let parts = parse_parts(str)?;

        let pattern = Pattern {
            id: PatternId::new(),
            precedence,
            parts,
        };
        Ok(pattern)
    })
}

fn parse_parts(str: &mut Stream) -> ParseResult<Vec<(Option<Ustr>, PatternPart)>> {
    let mut parts = Vec::new();

    let first = parse_part(str)?;
    parts.push(first);

    loop {
        match parse_part(str) {
            Ok(p) => parts.push(p),
            Err(ParseError::Backtrack(_)) => break,
            Err(ParseError::Commit(e)) => return Err(ParseError::Commit(e)),
        }
    }

    Ok(parts)
}

fn parse_part(str: &mut Stream) -> ParseResult<(Option<Ustr>, PatternPart)> {
    str.fallible(|str| {
        let name = if let Ok(name) = parse_name(str) {
            let name = match name.as_str() {
                "sentence" => return Ok((None, PatternPart::Sentence)),
                "value" => return Ok((None, PatternPart::Value)),
                "binding" => return Ok((None, PatternPart::Binding)),
                _ => Some(name),
            };
            str.expect_char(':')?;
            name
        } else {
            None
        };

        let part = match parse_name(str).map(|s| s.as_str()) {
            Ok("sentence") => PatternPart::Sentence,
            Ok("value") => PatternPart::Value,
            Ok("binding") => PatternPart::Binding,
            Ok(_) => str.fail()?,
            Err(_) => parse_lit(str)?,
        };

        Ok((name, part))
    })
}

fn parse_lit(str: &mut Stream) -> ParseResult<PatternPart> {
    str.expect_char('\'')?;

    str.commit(|str| {
        let mut lit = String::new();
        lit.push(str.expect_char_is(is_lit_char)?);
        while let Ok(next) = str.expect_char_is(is_lit_char) {
            lit.push(next);
        }

        str.expect_char('\'')?;
        Ok(PatternPart::Lit(Ustr::from(&lit)))
    })
}

fn is_lit_char(char: char) -> bool {
    char != '\''
}
