use std::time::Instant;

use ustr::Ustr;

use crate::{
    diagnostics::{ReportTracker, WResult},
    parse::{parsing_rules::extract_parsing_rules},
    statements::StatementsSet,
};

mod parsing_rules;

pub fn parse(ss: StatementsSet, tracker: &mut ReportTracker) -> WResult<()> {
    let rules = extract_parsing_rules(&ss, tracker)?;
    dbg!(rules);

    Ok(())
}

