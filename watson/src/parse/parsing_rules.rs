//! The goal of this module is to extract all the declared patterns from
//! syntax declarations, notation declarations, and definition declarations.
//! This allows us to parse sentences and values and thus fully parse all
//! source files.

use crate::{
    diagnostics::{ReportTracker, WResult, specifics},
    parse::utils::{ctx_char, ctx_desc, ctx_str},
    statements::{StatementTy, StatementsSet},
};
use tap::Pipe;
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct ParsingRules {
    sentence_patterns: Vec<Pattern>,
    value_patterns: Vec<Pattern>,
}

#[derive(Debug, Clone)]
struct Pattern {
    precedence: u32,
    parts: Vec<PatternPart>,
}

#[derive(Debug, Clone, Copy)]
enum PatternPart {
    Lit(Ustr),
    Sentence,
    Binding,
    Value,
}

pub fn extract_parsing_rules(
    ss: &StatementsSet,
    tracker: &mut ReportTracker,
) -> WResult<ParsingRules> {
    let mut sentence_patterns = Vec::new();
    let mut value_patterns = Vec::new();

    extract_from_syntax(ss, &mut sentence_patterns, tracker);
    extract_from_notation(ss, &mut sentence_patterns, tracker);
    extract_from_definition(ss, &mut value_patterns, tracker);

    tracker.checkpoint()?;

    Ok(ParsingRules {
        sentence_patterns,
        value_patterns,
    })
}

fn extract_from_syntax(
    ss: &StatementsSet,
    patterns: &mut Vec<Pattern>,
    tracker: &mut ReportTracker,
) {
    for decl in ss.statements_with_ty(StatementTy::Syntax) {
        match parse_syntax.parse(decl.text().as_str()) {
            Ok(p) => patterns.extend(p),
            Err(err) => tracker.add_message(specifics::parse_error(err, decl.span())),
        }
    }
}

fn extract_from_notation(
    ss: &StatementsSet,
    patterns: &mut Vec<Pattern>,
    tracker: &mut ReportTracker,
) {
    for decl in ss.statements_with_ty(StatementTy::Notation) {
        match parse_notation.parse(decl.text().as_str()) {
            Ok(p) => patterns.push(p),
            Err(err) => tracker.add_message(specifics::parse_error(err, decl.span())),
        }
    }
}

fn extract_from_definition(
    ss: &StatementsSet,
    patterns: &mut Vec<Pattern>,
    tracker: &mut ReportTracker,
) {
    for decl in ss.statements_with_ty(StatementTy::Definition) {
        match parse_definition.parse(decl.text().as_str()) {
            Ok(p) => patterns.push(p),
            Err(err) => tracker.add_message(specifics::parse_error(err, decl.span())),
        }
    }
}

use winnow::{
    ascii::{digit1, line_ending, multispace0},
    combinator::{alt, delimited, fail, opt, repeat, terminated},
    error::StrContext,
    prelude::*,
    token::{one_of, take_until, take_while},
};

fn parse_syntax<'a>(str: &mut &str) -> winnow::ModalResult<Vec<Pattern>> {
    "syntax".pipe(ws_nl).parse_next(str)?;
    let pattern = repeat(1.., terminated(parse_pattern, newline)).parse_next(str)?;
    "end".pipe(ws_nl).context(ctx_str("end")).parse_next(str)?;

    Ok(pattern)
}

fn parse_notation<'a>(str: &mut &str) -> winnow::ModalResult<Pattern> {
    "notation".pipe(ws_nl).parse_next(str)?;
    let _name = opt(parse_name).parse_next(str)?;
    let pattern = parse_pattern.parse_next(str)?;

    "=>".pipe(ws_nl).context(ctx_str("=>")).parse_next(str)?;
    take_until(0.., "end").parse_next(str)?;
    "end".pipe(ws_nl).context(ctx_str("end")).parse_next(str)?;

    Ok(pattern)
}

fn parse_definition<'a>(str: &mut &str) -> winnow::ModalResult<Pattern> {
    "definition".pipe(ws_nl).parse_next(str)?;
    let _name = opt(parse_name).parse_next(str)?;
    let pattern = parse_pattern.parse_next(str)?;

    alt(("=>", "where"))
        .pipe(ws_nl)
        .context(ctx_str("=>"))
        .context(ctx_str("where"))
        .parse_next(str)?;
    take_until(0.., "end").parse_next(str)?;
    "end".pipe(ws_nl).context(ctx_str("end")).parse_next(str)?;

    Ok(pattern)
}

fn parse_pattern(str: &mut &str) -> winnow::ModalResult<Pattern> {
    let precedence: u32 = digit1
        .pipe(ws_nl)
        .parse_to()
        .context(ctx_desc("precedence level"))
        .parse_next(str)?;

    "|".pipe(ws_nl).context(ctx_char('|')).parse_next(str)?;

    let parts = repeat(1.., parse_pattern_part)
        .context(StrContext::Label("pattern"))
        .parse_next(str)?;

    Ok(Pattern { precedence, parts })
}

fn parse_pattern_part(str: &mut &str) -> winnow::ModalResult<PatternPart> {
    opt((parse_name, ":".pipe(ws))).parse_next(str)?;

    alt((
        "sentence".value(PatternPart::Sentence),
        "binding".value(PatternPart::Binding),
        "value".value(PatternPart::Value),
        parse_lit,
        fail,
    ))
    .pipe(ws)
    .context(ctx_desc("`sentence`, `binding`, `value`, or literal"))
    .parse_next(str)
}

fn parse_lit(str: &mut &str) -> winnow::ModalResult<PatternPart> {
    take_until(1.., '\'')
        .take()
        .map(|s| PatternPart::Lit(Ustr::from(s)))
        .pipe(|p| delimited('\'', p, '\''))
        .parse_next(str)
}

pub fn parse_name(str: &mut &str) -> winnow::ModalResult<Ustr> {
    (
        one_of(|c: char| c.is_alpha() || c == '_'),
        take_while(0.., |c: char| c.is_alphanum() || c == '_'),
    )
        .take()
        .map(Ustr::from)
        .pipe(ws_nl)
        .parse_next(str)
}


pub fn newline<'s>(str: &mut &str) -> winnow::ModalResult<()> {
    take_while(0.., (' ', '\t', '\r')).parse_next(str)?;
    line_ending.parse_next(str)?;
    multispace0.parse_next(str)?;
    Ok(())
}

pub fn ws<'a, F, O, E: winnow::error::ParserError<&'a str>>(inner: F) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    let ws = (' ', '\t', '\r');
    delimited(take_while(0.., ws), inner, take_while(0.., ws))
}

pub fn ws_nl<'a, F, O, E: winnow::error::ParserError<&'a str>>(
    inner: F,
) -> impl Parser<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}