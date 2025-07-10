use winnow::Stateful;

use crate::{
    diagnostics::{ReportTracker, WResult},
    parse::parsing_rules::{ParsingRules, extract_parsing_rules},
    statements::StatementsSet,
};

mod axiom;
mod common;
mod parsing_rules;
mod sentence;
mod tactic;
mod theorem;
mod utils;
mod definition;

pub fn parse(ss: StatementsSet, tracker: &mut ReportTracker) -> WResult<()> {
    let rules = extract_parsing_rules(&ss, tracker)?;
    dbg!(rules);

    Ok(())
}

type Stream<'s> = Stateful<&'s str, State>;

#[derive(Debug, Clone)]
struct State {
    parsing_rules: ParsingRules,
}
