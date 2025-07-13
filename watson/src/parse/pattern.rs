use crate::{
    parse::stream::{ParseResult, Stream},
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternId(usize);

#[derive(Debug)]
pub struct Precedence(u32);

#[derive(Debug)]
pub struct Pattern {
    pub id: PatternId,
    pub statement: StatementId,
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
    todo!()
}
