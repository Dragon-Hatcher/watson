use crate::parse::{
    common::{parse_name, parse_num},
    stream::{ParseError, ParseResult, Stream},
};
use std::sync::atomic::{AtomicUsize, Ordering};
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
    pub ty: PatternTy,
    pub precedence: Precedence,
    pub parts: Vec<(Option<Ustr>, PatternPart)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternPart {
    Sentence,
    Value,
    Binding,
    Lit(Ustr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternTy {
    Sentence,
    Value,
}

pub fn parse_pattern(str: &mut Stream, ty: PatternTy) -> ParseResult<Pattern> {
    str.fallible(|str| {
        let precedence = Precedence(parse_num(str)?);
        str.expect_char('|')?;
        let parts = parse_parts(str)?;

        let pattern = Pattern {
            id: PatternId::new(),
            ty,
            precedence,
            parts,
        };
        Ok(pattern)
    })
}

fn parse_parts(str: &mut Stream) -> ParseResult<Vec<(Option<Ustr>, PatternPart)>> {
    let mut parts = Vec::new();

    let first = parse_name_and_part(str)?;
    parts.push(first);

    loop {
        match parse_name_and_part(str) {
            Ok(p) => parts.push(p),
            Err(ParseError::Backtrack(_)) => break,
            Err(ParseError::Commit(e)) => return Err(ParseError::Commit(e)),
        }
    }

    Ok(parts)
}

fn parse_name_and_part(str: &mut Stream) -> ParseResult<(Option<Ustr>, PatternPart)> {
    str.fallible(|str| {
        let name = parse_name(str)?;
        str.expect_char(':')?;
        let part = parse_part(str)?;

        Ok((Some(name), part))
    })
    .or_else(|_| {
        let part = parse_part(str)?;
        Ok((None, part))
    })
}

fn parse_part(str: &mut Stream) -> ParseResult<PatternPart> {
    if str.expect_str("sentence").is_ok() {
        Ok(PatternPart::Sentence)
    } else if str.expect_str("value").is_ok() {
        Ok(PatternPart::Value)
    } else if str.expect_str("binding").is_ok() {
        Ok(PatternPart::Binding)
    } else {
        parse_lit(str)
    }
}

fn parse_lit(str: &mut Stream) -> ParseResult<PatternPart> {
    str.include_ws(|str| {
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
    })
}

fn is_lit_char(char: char) -> bool {
    char != '\''
}
