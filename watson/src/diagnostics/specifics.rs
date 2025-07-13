use super::ReportLevel as RL;
use crate::{
    diagnostics::Report,
    parse::stream::{ParseError, ParseErrorCtxTy},
    span::Span,
    statements::StatementTy,
};
use ustr::Ustr;

macro_rules! uformat {
    ($($t:tt)*) => {
        ustr::Ustr::from(&format!($($t)*))
    };
}

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

pub fn parse_error(err: ParseError, span: Span) -> Report {
    let err = match err {
        ParseError::Backtrack(err) => err,
        ParseError::Commit(err) => err,
    };

    let span = Span::new(
        span.file(),
        span.start() + err.place(),
        span.start() + err.place() + 1,
    );

    let mut msg = String::new();
    for part in err.trace() {
        if !msg.is_empty() {
            msg.push_str(" ");
        }

        match part {
            ParseErrorCtxTy::ExpectChar(c) => msg.push_str(&format!("expected `{c}`")),
            ParseErrorCtxTy::ExpectStr(str) => msg.push_str(&format!("expected `{str}`")),
            ParseErrorCtxTy::ExpectDescription(desc) => msg.push_str(&format!("expected {desc}")),
            ParseErrorCtxTy::Label(label) => msg.push_str(&format!("while parsing {label}")),
        }
    }

    if msg.is_empty() {
        msg.push_str("error here");
    }

    let msg = Ustr::from(&msg);

    Report::new(RL::Error, uformat!("parse error")).with_info(span, msg)
}
