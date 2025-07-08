use crate::util::line_ranges;

pub fn parse(file: &str) {
    let statements = split_statements(file);
    dbg!(statements);
}

fn split_statements<'a>(file: &'a str) -> Vec<Statement<'a>> {
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
                    let prose_slice = &file[current_start..start_idx].trim();
                    if !prose_slice.is_empty() {
                        statements.push(Statement {
                            ty: StatementTy::Prose,
                            text: prose_slice,
                        });
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
                let statement_slice = &file[start..end_idx].trim();
                statements.push(Statement {
                    ty: statement_ty,
                    text: statement_slice,
                });

                // Reset back to prose mode.
                current_delims = None;
                current_start = None;
            }
        }
    }

    statements
}

#[derive(Debug, Clone, Copy)]
struct Statement<'a> {
    ty: StatementTy,
    text: &'a str,
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
