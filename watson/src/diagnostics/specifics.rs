macro_rules! uformat {
        ($($t:tt)*) => {
            ustr::Ustr::from(&format!($($t)*))
        };
    }

use ustr::Ustr;
use winnow::error::{ContextError, ParseError};

use super::ReportLevel as RL;
use crate::{diagnostics::Report, span::Span, statements::StatementTy};

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
    .with_info(
        span,
        uformat!("{} was opened here", render_statement_ty(ty)),
    )
    .with_info(next_span, uformat!("and was still open here"))
}

pub fn unclosed_statement_at_eof(span: Span, ty: StatementTy) -> Report {
    Report::new(
        RL::Error,
        uformat!(
            "{} declaration was unclosed at EOF",
            render_statement_ty(ty)
        ),
    )
    .with_info(
        span,
        uformat!(
            "{} was opened here and was still open at EOF",
            render_statement_ty(ty)
        ),
    )
}

pub fn multiple_syntax_statements(mut spans: impl Iterator<Item = Span>) -> Report {
    let mut r = Report::new(RL::Error, uformat!("multiple syntax declarations"));

    r = r.with_info(spans.next().unwrap(), uformat!("syntax declared here"));
    for span in spans {
        r = r.with_info(span, uformat!("and here"))
    }

    r
}

pub fn parse_error(err: ParseError<&str, ContextError>, span: Span) -> Report {
    let span = Span::new(
        span.file(),
        span.start() + err.offset(),
        span.start() + err.offset() + 1,
    );
    let msg = Ustr::from(&err.inner().to_string());

    Report::new(RL::Error, uformat!("parsing error")).with_info(span, msg)
}
