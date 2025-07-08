use ariadne::{Config, Label, Report, ReportKind};
use ustr::Ustr;

use crate::{
    span::{SourceCache, Span},
    util::line_ranges,
};

pub fn parse(sources: &SourceCache, filename: Ustr) {
    let statements = split_statements(sources, filename);

    let statement = statements[1];

    Report::build(ReportKind::Error, statement.span)
        .with_config(Config::new().with_index_type(ariadne::IndexType::Byte).with_char_set(ariadne::CharSet::Ascii))
        .with_message("This is a syntax declaration.")
        .with_label(
            Label::new(statement.span)
                .with_message("This is the code for it.")
                .with_color(ariadne::Color::BrightBlue),
        )
        .finish()
        .print(sources)
        .unwrap();
}

fn split_statements(sources: &SourceCache, filename: Ustr) -> Vec<Statement> {
    type Delims = (&'static str, &'static str, StatementTy);

    const STATEMENT_DELIMITERS: [Delims; 5] = [
        ("syntax", "end", StatementTy::Syntax),
        ("notation", "end", StatementTy::Notation),
        ("definition", "end", StatementTy::Definition),
        ("theorem", "qed", StatementTy::Theorem),
        ("axiom", "end", StatementTy::Axiom),
    ];

    let file = sources.get_text(filename);

    let mut statements = Vec::new();
    let mut current_delims = None;
    let mut current_start = None;

    let make_statement = |ty: StatementTy, start: usize, end: usize| Statement {
        ty,
        text: Ustr::from(&file[start..end].trim()),
        span: Span::new(filename, start, end),
    };

    for (start_idx, end_idx) in line_ranges(file) {
        let line = &file[start_idx..end_idx];

        if current_delims.is_none() {
            // We are currently in prose. Check if we are opening a statement or
            // if the prose continues.

            for delim in STATEMENT_DELIMITERS {
                if !line.starts_with(delim.0) {
                    continue;
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

    statements
}

#[derive(Debug, Clone, Copy)]
struct Statement {
    ty: StatementTy,
    span: Span,
    text: Ustr,
}

#[derive(Debug, Clone, Copy)]
enum StatementTy {
    Prose,
    Syntax,
    Notation,
    Definition,
    Axiom,
    Theorem,
}
