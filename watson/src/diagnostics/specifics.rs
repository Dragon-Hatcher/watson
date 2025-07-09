macro_rules! uformat {
        ($($t:tt)*) => {
            ustr::Ustr::from(&format!($($t)*))
        };
    }

use super::ReportLevel as RL;
use crate::{diagnostics::Report, parser::StatementTy, span::Span};

fn render_statement_ty(ty: StatementTy) -> &'static str {
    match ty {
        StatementTy::Prose => "prose",
        StatementTy::Syntax => "syntax",
        StatementTy::Notation => "notation",
        StatementTy::Definition => "definition",
        StatementTy::Axiom => "axiom",
        StatementTy::Theorem => "theorem",
    }
}

pub fn unclosed_statement(
    span: Span,
    ty: StatementTy,
    next_span: Span,
    next_ty: StatementTy,
) -> Report {
    Report::new(
        RL::Error,
        uformat!(
            "{} declaration was unclosed at following {} declaration",
            render_statement_ty(ty),
            render_statement_ty(next_ty)
        ),
    )
        .with_info(span, uformat!("{} was opened here", render_statement_ty(ty)))
        .with_info(next_span, uformat!("and was still open here"))
}

pub fn unclosed_statement_at_eof(span: Span, ty: StatementTy) -> Report {
    Report::new(
        RL::Error,
        uformat!(
            "{} declaration was unclosed at EOF",
            render_statement_ty(ty)
        ),
    ).with_info(span, uformat!("{} was opened here and was still open at EOF", render_statement_ty(ty)))
}
