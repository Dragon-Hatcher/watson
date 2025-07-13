use ustr::Ustr;

use crate::{
    parse::{
        find_patterns::PatternArena,
        stream::{ParseResult, Stream},
    },
    statements::StatementId,
};

#[derive(Debug)]
pub struct PatternId(usize);

#[derive(Debug)]
pub struct Precedence(u32);

#[derive(Debug)]
pub struct Pattern {
    id: PatternId,
    statement: StatementId,
    precedence: Precedence,
    parts: Vec<(Option<Ustr>, PatternPart)>,
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

pub fn parse_pattern(str: &mut Stream, arena: &mut PatternArena) -> ParseResult<PatternId> {
    todo!()
}
