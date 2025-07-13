use ustr::Ustr;

use crate::parse::{
    pattern::PatternId,
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

pub fn parse_value(str: &mut Stream) -> ParseResult<Value> {
    todo!()
}
