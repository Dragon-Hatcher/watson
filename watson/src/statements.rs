use crate::{
    diagnostics::{ReportTracker, WResult, specifics},
    span::{Filename, SourceCache, Span},
    util::line_ranges,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use ustr::Ustr;

pub struct StatementsSet {
    statements: Vec<Statement>,
}

impl StatementsSet {
    fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    pub fn statements(&self) -> impl Iterator<Item = &Statement> {
        self.statements.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatementId(usize);

impl StatementId {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::SeqCst);
        Self(id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Statement {
    id: StatementId,
    ty: StatementTy,
    span: Span,
    text: Ustr,
}

impl Statement {
    pub fn ty(&self) -> StatementTy {
        self.ty
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn text(&self) -> Ustr {
        self.text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatementTy {
    Prose,
    Syntax,
    Notation,
    Definition,
    Axiom,
    Theorem,
}

pub fn get_all_statements(
    sources: &SourceCache,
    tracker: &mut ReportTracker,
) -> WResult<StatementsSet> {
    let mut ss = StatementsSet::new();

    for (&filename, text) in sources.files() {
        extract_statements(&mut ss, filename, text, tracker);
    }

    tracker.checkpoint()?;
    Ok(ss)
}

fn extract_statements(
    ss: &mut StatementsSet,
    filename: Filename,
    text: &str,
    tracker: &mut ReportTracker,
) {
    type Delims = (&'static str, &'static str, StatementTy);

    const STATEMENT_DELIMITERS: [Delims; 5] = [
        ("syntax", "end", StatementTy::Syntax),
        ("notation", "end", StatementTy::Notation),
        ("definition", "end", StatementTy::Definition),
        ("theorem", "qed", StatementTy::Theorem),
        ("axiom", "end", StatementTy::Axiom),
    ];

    let mut current_delims: Option<(&'static str, &'static str, StatementTy)> = None;
    let mut current_start = None;

    let make_span = |start: usize, end: usize| Span::new(filename, start, end);
    let make_statement = |ty: StatementTy, start: usize, end: usize| Statement {
        id: StatementId::new(),
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

            if let Some((start_text, _, ty)) = current_delims
                && let Some(start) = current_start
            {
                // We were already in a statement so it must not have been
                // properly closed. Close that statement and throw an error.
                tracker.add_message(specifics::unclosed_statement(
                    make_span(start, start + start_text.len()),
                    ty,
                    make_span(start_idx, start_idx + delim.0.len()),
                    delim.2,
                ));
            }

            // First, if we have ongoing prose, save it.
            if let Some(current_start) = current_start {
                let statement = make_statement(StatementTy::Prose, current_start, start_idx);
                if !statement.text.is_empty() {
                    ss.add_statement(statement);
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
                ss.add_statement(statement);

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
        ss.add_statement(make_statement(StatementTy::Prose, start, text.len()));
    }

    if let Some(start) = current_start
        && let Some((start_text, _, ty)) = current_delims
    {
        tracker.add_message(specifics::unclosed_statement_at_eof(
            make_span(start, start + start_text.len()),
            ty,
        ));
    }
}
