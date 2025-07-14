use super::term::Term;
use crate::parse::{
    Document,
    common::{parse_kw, parse_name},
    stream::{ParseError, ParseResult, Stream},
    term::{Sentence, Value, parse_sentence, parse_sentence_or_value},
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Proof {
    tactics: Vec<Tactic>,
}

#[derive(Debug)]
pub enum Tactic {
    Have(HaveTactic),
}

#[derive(Debug)]
pub struct HaveTactic {
    conclusion: Sentence,
    by: Ustr,
    substitutions: Vec<Substitution>,
}

#[derive(Debug)]
pub enum Substitution {
    Value(Value),
    Sentence(Sentence),
}

pub fn parse_proof(str: &mut Stream, doc: &Document) -> ParseResult<Proof> {
    let mut tactics = Vec::new();

    loop {
        match parse_tactic(str, doc) {
            Ok(s) => tactics.push(s),
            Err(ParseError::Backtrack(_)) => break,
            Err(ParseError::Commit(e)) => return Err(ParseError::Commit(e)),
        }
    }

    Ok(Proof { tactics })
}

fn parse_tactic(str: &mut Stream, doc: &Document) -> ParseResult<Tactic> {
    str.fallible(|str| {
        parse_kw(str, "have")?;
        let conclusion = parse_sentence(str, "by", doc)?;
        let by = parse_name(str)?;
        let substitutions = parse_substitutions(str, doc)?;

        Ok(Tactic::Have(HaveTactic {
            conclusion,
            by,
            substitutions,
        }))
    })
}

fn parse_substitutions(str: &mut Stream, doc: &Document) -> ParseResult<Vec<Substitution>> {
    let mut substitutions = Vec::new();

    loop {
        match parse_substitution(str, doc) {
            Ok(s) => substitutions.push(s),
            Err(ParseError::Backtrack(_)) => break,
            Err(ParseError::Commit(e)) => return Err(ParseError::Commit(e)),
        }
    }

    Ok(substitutions)
}

fn parse_substitution(str: &mut Stream, doc: &Document) -> ParseResult<Substitution> {
    str.fallible(|str| {
        str.expect_char('[')?;
        let term = parse_sentence_or_value(str, "]", doc)?;

        let sub = match term {
            Term::Sentence(sentence) => Substitution::Sentence(sentence),
            Term::Value(value) => Substitution::Value(value),
        };
        Ok(sub)
    })
}
