use crate::{diagnostics::{ReportTracker, WResult}, parse::syntax::get_syntax, statements::StatementsSet};

mod syntax;

pub fn parse(ss: StatementsSet, tracker: &mut ReportTracker) -> WResult<()> {
    let syntax = get_syntax(&ss, tracker)?;
    dbg!(syntax);
    
    tracker.checkpoint()?;
    Ok(())
}