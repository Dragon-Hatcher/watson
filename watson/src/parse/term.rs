use super::pattern::PatternPart;
use crate::parse::{
    Document,
    common::parse_name,
    earley::{EarleyGrammar, EarleySymbol, EarleyTerm, earley_parse},
    find_patterns::PatternArena,
    pattern::{Pattern, PatternId, PatternTy},
    stream::{Checkpoint, ParseResult, Stream},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyEarleyNonTerm {
    Sentence,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyEarleyTerm {
    Lit(Ustr),
    Binding,
    VarName,
    PatSubst,
    DefSubst,
}

impl EarleyTerm for MyEarleyTerm {
    fn scan(&self, str: &mut Stream) -> Option<Checkpoint> {
        str.measure(|str| {
            match self {
                MyEarleyTerm::Lit(ustr) => {
                    str.expect_str(ustr.as_str())?;
                }
                MyEarleyTerm::Binding | MyEarleyTerm::VarName => {
                    parse_name(str)?;
                }
                MyEarleyTerm::PatSubst => {
                    str.include_ws(|str| {
                        str.expect_char('$')?;
                        parse_name(str)
                    })?;
                }
                MyEarleyTerm::DefSubst => {
                    str.expect_char('_')?;
                }
            };
            Ok(())
        })
    }
}

fn pattern_to_earley(pat: &Pattern) -> Vec<EarleySymbol<MyEarleyTerm, MyEarleyNonTerm>> {
    pat.parts
        .iter()
        .map(|p| match p.1 {
            PatternPart::Sentence => EarleySymbol::NonTerminal(MyEarleyNonTerm::Sentence),
            PatternPart::Value => EarleySymbol::NonTerminal(MyEarleyNonTerm::Value),
            PatternPart::Binding => EarleySymbol::Terminal(MyEarleyTerm::Binding),
            PatternPart::Lit(ustr) => EarleySymbol::Terminal(MyEarleyTerm::Lit(ustr)),
        })
        .collect()
}

pub fn parse_sentence(str: &mut Stream, end: &str, doc: &Document) -> ParseResult<Sentence> {
    let mut grammar = EarleyGrammar::new();

    for pat in doc.patterns.patterns_with_ty(PatternTy::Sentence) {
        let pat = doc.patterns.get(pat);
        grammar.add_rule(MyEarleyNonTerm::Sentence, pattern_to_earley(pat));
    }
    grammar.add_rule(
        MyEarleyNonTerm::Sentence,
        vec![EarleySymbol::Terminal(MyEarleyTerm::PatSubst)],
    );

    for pat in doc.patterns.patterns_with_ty(PatternTy::Value) {
        let pat = doc.patterns.get(pat);
        grammar.add_rule(MyEarleyNonTerm::Value, pattern_to_earley(pat));
    }
    grammar.add_rule(
        MyEarleyNonTerm::Value,
        vec![EarleySymbol::Terminal(MyEarleyTerm::PatSubst)],
    );
    grammar.add_rule(
        MyEarleyNonTerm::Value,
        vec![EarleySymbol::Terminal(MyEarleyTerm::DefSubst)],
    );
    grammar.add_rule(
        MyEarleyNonTerm::Value,
        vec![EarleySymbol::Terminal(MyEarleyTerm::VarName)],
    );

    earley_parse(str, grammar, MyEarleyNonTerm::Sentence);

    todo!()
}

pub fn parse_value(str: &mut Stream, end: &str) -> ParseResult<Value> {
    todo!()
}

fn parse_default_values(str: &mut Stream) -> Option<Checkpoint> {
    str.measure(|str| parse_name(str)).or(str.measure(|str| {
        str.include_ws(|str| {
            str.expect_char('$')?;
            parse_name(str)
        })
    }))
}

fn parse_default_sentences(str: &mut Stream) -> Option<Checkpoint> {
    str.measure(|str| {
        str.include_ws(|str| {
            str.expect_char('$')?;
            parse_name(str)
        })
    })
}
