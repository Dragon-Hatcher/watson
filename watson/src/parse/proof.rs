use crate::parse::{
    stream::{ParseResult, Stream},
    term::{Sentence, Value},
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Proof {
    tactics: Vec<Tactic>,
}

#[derive(Debug)]
pub enum Tactic {
    Have(HaveTactic),
}

#[derive(Debug)]
pub struct HaveTactic {
    conclusion: Sentence,
    by: Ustr,
    substitutions: Vec<Substitution>,
}

#[derive(Debug)]
pub enum Substitution {
    Value(Value),
    Sentence(Sentence),
}

pub fn parse_proof(str: &mut Stream) -> ParseResult<Proof> {
    todo!()
}
