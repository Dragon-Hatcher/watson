use ustr::Ustr;

use crate::parse::{
    find_patterns::PatternId,
    stream::{ParseResult, Stream},
};

#[derive(Debug)]
pub struct Sentence {
    pattern: PatternId,
    terms: Vec<Term>,
}

#[derive(Debug)]
pub struct Value {
    pattern: PatternId,
    terms: Vec<Term>,
}

#[derive(Debug)]
pub enum Term {
    Sentence(Sentence),
    Value(Value),
    Binding(Ustr),
}

pub fn parse_sentence(str: &mut Stream) -> ParseResult<Sentence> {
    todo!()
}
