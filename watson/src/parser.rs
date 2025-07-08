use crate::{
    diagnostics::{specifics, ReportTracker, WResult},
    span::{Filename, SourceCache, Span},
    util::line_ranges,
};
use ustr::Ustr;

pub fn parse(sources: &SourceCache, tracker: &mut ReportTracker) -> WResult<Vec<Statement>> {
    let mut all_statements = Vec::new();

    for (&filename, text) in sources.files() {
        let statements = split_statements(filename, text, tracker);
        all_statements.extend(statements);
    }

    tracker.checkpoint()?;
    Ok(all_statements)
}

fn split_statements(filename: Filename, text: &str, tracker: &mut ReportTracker) -> Vec<Statement> {
    type Delims = (&'static str, &'static str, StatementTy);

    const STATEMENT_DELIMITERS: [Delims; 5] = [
        ("syntax", "end", StatementTy::Syntax),
        ("notation", "end", StatementTy::Notation),
        ("definition", "end", StatementTy::Definition),
        ("theorem", "qed", StatementTy::Theorem),
        ("axiom", "end", StatementTy::Axiom),
    ];

    let mut statements = Vec::new();
    let mut current_delims = None;
    let mut current_start = None;

    let make_span = |start: usize, end: usize| Span::new(filename, start, end);
    let make_statement = |ty: StatementTy, start: usize, end: usize| Statement {
        ty,
        text: Ustr::from(text[start..end].trim()),
        span: make_span(start, end),
    };

    for (start_idx, end_idx) in line_ranges(text) {
        let line = &text[start_idx..end_idx];

        // Check if this line opens a new statement.
        for delim in STATEMENT_DELIMITERS {
            if !line.starts_with(delim.0) {
                continue;
            }

            if let Some((_, _, ty)) = current_delims
                && let Some(start) = current_start
            {
                // We were already in a statement so it must not have been
                // properly closed. Close that statement and throw an error.
                tracker.add_message(specifics::unclosed_statement(make_span(start, start_idx), ty, delim.2));
            }

            // First, if we have ongoing prose, save it.
            if let Some(current_start) = current_start {
                let statement = make_statement(StatementTy::Prose, current_start, start_idx);
                if !statement.text.is_empty() {
                    statements.push(statement);
                }
            }

            // Now save the start of this statement
            current_delims = Some(delim);
            current_start = Some(start_idx);
            break;
        }

        if current_delims.is_none() && current_start.is_none() {
            // This is the beginning of prose and we need to reset the range start index.
            current_start = Some(start_idx);
        }

        if let Some((_, end, statement_ty)) = current_delims
            && let Some(start) = current_start
        {
            // We need to check for the end of a statement.
            if line.trim().ends_with(end) {
                // Make the statement and push it.
                let statement = make_statement(statement_ty, start, end_idx);
                statements.push(statement);

                // Reset back to prose mode.
                current_delims = None;
                current_start = None;
            }
        }
    }

    if let Some(start) = current_start
        && current_delims.is_none()
    {
        // The file ends with prose.
        statements.push(make_statement(StatementTy::Prose, start, text.len()));
    }

    if let Some(start) = current_start
        && let Some((_, _, ty)) = current_delims
    {
        tracker.add_message(specifics::unclosed_statement_at_eof(make_span(start, text.len()), ty));
    }

    statements
}

#[derive(Debug, Clone, Copy)]
pub struct Statement {
    ty: StatementTy,
    span: Span,
    text: Ustr,
}

#[derive(Debug, Clone, Copy)]
pub enum StatementTy {
    Prose,
    Syntax,
    Notation,
    Definition,
    Axiom,
    Theorem,
}
