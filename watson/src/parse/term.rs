use super::pattern::PatternPart;
use crate::parse::{
    Document,
    common::parse_name,
    earley::{EarleyRule, EarleySymbol, earley_parse},
    find_patterns::PatternArena,
    pattern::{Pattern, PatternId, PatternTy},
    stream::{ParseResult, Stream},
};
use std::{fmt::Debug, hash::Hash};
use ustr::Ustr;

#[derive(Debug)]
pub struct Sentence {
    pattern: PatternId,
    terms: Vec<Term>,
}

#[derive(Debug)]
pub struct Value {
    pattern: PatternId,
    terms: Vec<Term>,
}

#[derive(Debug)]
pub enum Term {
    Sentence(Sentence),
    Value(Value),
    Binding(Ustr),
}

#[derive(Clone, Copy)]
struct TermEarleyRule<'a> {
    patterns: &'a PatternArena,
    pat: &'a Pattern,
}

impl<'a> Debug for TermEarleyRule<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x: Vec<_> = self.pat.parts.iter().map(|x| x.1).collect();
        x.fmt(f)
    }
}

impl<'a> PartialEq for TermEarleyRule<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.pat as *const _ == other.pat as *const _
    }
}
impl<'a> Eq for TermEarleyRule<'a> {}
impl<'a> Hash for TermEarleyRule<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.pat as *const Pattern).hash(state);
    }
}

#[derive(Clone, Copy)]
struct TermEarleySymbol<'a> {
    patterns: &'a PatternArena,
    ty: PatternPart,
}

impl<'a> Debug for TermEarleySymbol<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.ty.fmt(f)
    }
}

impl<'a> PartialEq for TermEarleySymbol<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty
    }
}
impl<'a> Eq for TermEarleySymbol<'a> {}
impl<'a> Hash for TermEarleySymbol<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
    }
}

impl<'a> EarleyRule<TermEarleySymbol<'a>> for TermEarleyRule<'a> {
    fn predict(&self, pos: usize) -> Option<TermEarleySymbol<'a>> {
        self.pat.parts.get(pos).map(|&(_, ty)| TermEarleySymbol {
            patterns: self.patterns,
            ty,
        })
    }

    fn debug(&self) -> Vec<String> {
        self.pat
            .parts
            .iter()
            .map(|p| format!("{:?}", p.1))
            .collect()
    }
}

impl<'a> EarleySymbol<TermEarleyRule<'a>> for TermEarleySymbol<'a> {
    fn scan(&self, str: &mut Stream) -> Option<super::stream::Checkpoint> {
        match self.ty {
            PatternPart::Binding => str.measure(|str| parse_name(str)),
            PatternPart::Lit(lit) => str.measure(|str| str.expect_str(lit.as_str())),
            _ => None,
        }
    }

    fn rules_for(&self) -> impl Iterator<Item = TermEarleyRule<'a>> {
        let mapper = |pat: &PatternId| TermEarleyRule {
            patterns: self.patterns,
            pat: self.patterns.get(pat),
        };

        match self.ty {
            PatternPart::Sentence => self
                .patterns
                .patterns_with_ty(PatternTy::Sentence)
                .iter()
                .map(mapper),
            PatternPart::Value => self
                .patterns
                .patterns_with_ty(PatternTy::Value)
                .iter()
                .map(mapper),
            _ => [].iter().map(mapper),
        }
    }
}

pub fn parse_sentence(str: &mut Stream, end: &str, doc: &Document) -> ParseResult<Sentence> {
    earley_parse(
        str,
        TermEarleySymbol {
            patterns: &doc.patterns,
            ty: PatternPart::Sentence,
        },
    );

    todo!()
}

pub fn parse_value(str: &mut Stream, end: &str) -> ParseResult<Value> {
    todo!()
}
