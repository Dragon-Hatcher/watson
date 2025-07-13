use std::collections::HashMap;

use crate::{
    diagnostics::{ReportTracker, WResult},
    statements::{StatementId, StatementsSet},
};
use ustr::Ustr;

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
pub struct PatternArena {
    map: HashMap<PatternId, Pattern>,
}

#[derive(Debug)]
pub enum PatternTy {
    Sentence,
    Value,
}

#[derive(Debug)]
pub enum PatternPart {
    Sentence,
    Value,
    Binding,
    Lit(Ustr),
}

pub fn find_patterns(ss: &StatementsSet, tracker: &mut ReportTracker) -> WResult<PatternArena> {
    todo!()
}
