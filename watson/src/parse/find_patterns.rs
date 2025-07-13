use std::collections::HashMap;

use crate::{
    diagnostics::{ReportTracker, WResult},
    parse::pattern::{Pattern, PatternId},
    statements::StatementsSet,
};

#[derive(Debug)]
pub struct PatternArena {
    map: HashMap<PatternId, Pattern>,
}

pub fn find_patterns(ss: &StatementsSet, tracker: &mut ReportTracker) -> WResult<PatternArena> {
    todo!()
}
