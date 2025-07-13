use std::collections::HashMap;

use crate::{
    diagnostics::{ReportTracker, WResult},
    statements::{StatementId, StatementsSet},
};
use ustr::Ustr;

pub struct PatternId(usize);

pub struct Precedence(u32);

pub struct Pattern {
    id: PatternId,
    statement: StatementId,
    precedence: Precedence,
    parts: Vec<(Option<Ustr>, PatternPart)>,
}

pub struct PatternArena {
    map: HashMap<PatternId, Pattern>,
}

pub enum PatternTy {
    Sentence,
    Value,
}

pub enum PatternPart {
    Sentence,
    Value,
    Binding,
    Lit(Ustr),
}

pub fn find_patterns(ss: &StatementsSet, tracker: &mut ReportTracker) -> WResult<()> {
    Ok(())
}
