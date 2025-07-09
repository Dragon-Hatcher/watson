use crate::{
    diagnostics::{ReportTracker, WResult, specifics},
    span::Span,
    statements::{Statement, StatementTy, StatementsSet},
};
use tap::Pipe;
use ustr::Ustr;

#[derive(Debug, Clone, Copy)]
pub enum SentenceSyntaxPart {
    Lit(Ustr),
    Sentence,
    Binding,
    Value,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    precedence: u32,
    parts: Vec<SentenceSyntaxPart>,
}

#[derive(Debug, Clone)]
pub struct SentenceSyntax {
    patterns: Vec<Pattern>,
}

pub fn get_syntax(ss: &StatementsSet, tracker: &mut ReportTracker) -> WResult<SentenceSyntax> {
    let syntax_statements = ss.statements_with_ty(StatementTy::Syntax);

    let [syntax_statement] = syntax_statements else {
        tracker.add_message(specifics::multiple_syntax_statements(
            syntax_statements.iter().map(Statement::span),
        ));
        return Err(());
    };

    parse_syntax(
        syntax_statement.text().as_str(),
        syntax_statement.span(),
        tracker,
    )
}

use winnow::{
    ascii::{digit1, line_ending, multispace0},
    combinator::{self, cut_err, fail, not, repeat},
    error::{StrContext, StrContextValue},
    prelude::*,
    token::{literal, none_of},
};

fn parse_syntax(str: &str, span: Span, tracker: &mut ReportTracker) -> WResult<SentenceSyntax> {
    match winnow_parse.parse(str) {
        Ok(s) => Ok(s),
        Err(err) => {
            tracker.add_message(specifics::parse_error(err, span));
            Err(())
        }
    }
}

fn winnow_parse<'s>(str: &mut &'s str) -> winnow::ModalResult<SentenceSyntax> {
    combinator::seq! {SentenceSyntax{
        _: "syntax".pipe(ws_nl),
        patterns: combinator::separated(1.., parse_pattern, newline),
        _: "end".pipe(ws_nl),
    }}
    .parse_next(str)
}

const PRECEDENCE_CTX: StrContext =
    StrContext::Expected(StrContextValue::Description("precedence level"));
const PIPE_CTX: StrContext = StrContext::Expected(StrContextValue::CharLiteral('|'));
const PART_CTX: StrContext = StrContext::Expected(StrContextValue::Description(
    "`sentence`, `binding`, `value`, or a literal",
));

fn parse_pattern<'s>(str: &mut &'s str) -> winnow::ModalResult<Pattern> {
    // Check if we are committed to parsing a pattern
    not("end").parse_next(str)?;

    combinator::seq! {Pattern {
        precedence: digit1.parse_to().pipe(ws).context(PRECEDENCE_CTX),
        _: "|".pipe(ws).context(PIPE_CTX),
        parts: repeat(1.., parse_syntax_part),
    }}
    .pipe(cut_err)
    .parse_next(str)
}

fn parse_syntax_part<'s>(str: &mut &'s str) -> winnow::ModalResult<SentenceSyntaxPart> {
    combinator::alt((
        parse_lit,
        "sentence".value(SentenceSyntaxPart::Sentence),
        "binding".value(SentenceSyntaxPart::Binding),
        "value".value(SentenceSyntaxPart::Value),
        fail,
    ))
    .pipe(ws)
    .context(PART_CTX)
    .parse_next(str)
}

fn parse_lit<'s>(str: &mut &'s str) -> winnow::ModalResult<SentenceSyntaxPart> {
    '\''.parse_next(str)?;
    let part = repeat(1.., none_of('\''))
        .fold(String::new, |mut s, c| {
            s.push(c);
            s
        })
        .map(|s| SentenceSyntaxPart::Lit(Ustr::from(&s)))
        .parse_next(str)?;
    '\''.parse_next(str)?;

    Ok(part)
}

fn newline<'s>(str: &mut &'s str) -> winnow::ModalResult<()> {
    winnow::token::take_while(0.., (' ', '\t', '\r')).parse_next(str)?;
    line_ending.parse_next(str)?;
    multispace0.parse_next(str)?;
    Ok(())
}

fn ws<'a, F, O, E: winnow::error::ParserError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    combinator::delimited(
        winnow::token::take_while(0.., (' ', '\t', '\r')),
        inner,
        winnow::token::take_while(0.., (' ', '\t', '\r')),
    )
}

fn ws_nl<'a, F, O, E: winnow::error::ParserError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    combinator::delimited(multispace0, inner, multispace0)
}
