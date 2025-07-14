use super::pattern::PatternPart;
use crate::parse::{
    Document,
    common::{can_start_name, parse_kw, parse_name, parse_schema_name},
    earley::{EarleyGrammar, EarleySymbol, EarleyTerm, earley_parse},
    pattern::{Pattern, PatternId, PatternTy},
    stream::{Checkpoint, ParseResult, Stream},
};
use std::{
    fmt::Debug,
    hash::Hash,
    time::{Duration, Instant},
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Sentence {
    // pattern: PatternId,
    // terms: Vec<Term>,
}

#[derive(Debug)]
pub struct Value {
    // pattern: PatternId,
    // terms: Vec<Term>,
}

#[derive(Debug)]
pub enum Term {
    Sentence(Sentence),
    Value(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyEarleyNonTerm {
    StartSym,
    Sentence,
    Value,
    Binding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MyEarleyTerm {
    Lit(Ustr),
    SchemaName,
    VarName,
    PatSubst,
    DefSubst,
    Kw(Ustr),
}

impl EarleyTerm for MyEarleyTerm {
    fn scan(&self, str: &mut Stream) -> Option<Checkpoint> {
        str.measure(|str| {
            match self {
                MyEarleyTerm::Lit(ustr) => {
                    str.expect_str(ustr.as_str())?;
                }
                MyEarleyTerm::VarName => {
                    parse_name(str)?;
                }
                MyEarleyTerm::SchemaName => {
                    parse_schema_name(str)?;
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
                MyEarleyTerm::Kw(kw) => {
                    parse_kw(str, kw)?;
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
            PatternPart::Binding => EarleySymbol::NonTerminal(MyEarleyNonTerm::Binding),
            PatternPart::Lit(ustr) => EarleySymbol::Terminal(MyEarleyTerm::Lit(ustr)),
        })
        .collect()
}

fn end_term(end: &str) -> MyEarleyTerm {
    if can_start_name(end.chars().next().unwrap()) {
        MyEarleyTerm::Kw(Ustr::from(end))
    } else {
        MyEarleyTerm::Lit(Ustr::from(end))
    }
}

fn build_grammar(doc: &Document) -> EarleyGrammar<MyEarleyTerm, MyEarleyNonTerm> {
    let mut grammar = EarleyGrammar::new();

    for pat in doc.patterns.patterns_with_ty(PatternTy::Sentence) {
        let pat = doc.patterns.get(pat);
        grammar.add_rule(MyEarleyNonTerm::Sentence, pattern_to_earley(pat));
    }
    grammar.add_rule(
        MyEarleyNonTerm::Sentence,
        vec![EarleySymbol::Terminal(MyEarleyTerm::PatSubst)],
    );
    grammar.add_rule(
        MyEarleyNonTerm::Sentence,
        vec![EarleySymbol::Terminal(MyEarleyTerm::SchemaName)],
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

    grammar.add_rule(
        MyEarleyNonTerm::Binding,
        vec![EarleySymbol::Terminal(MyEarleyTerm::VarName)],
    );
    grammar.add_rule(
        MyEarleyNonTerm::Binding,
        vec![EarleySymbol::Terminal(MyEarleyTerm::PatSubst)],
    );

    grammar
}

pub fn parse_sentence(str: &mut Stream, end: &str, doc: &Document) -> ParseResult<Sentence> {
    let mut grammar = build_grammar(doc);
    grammar.add_rule(
        MyEarleyNonTerm::StartSym,
        vec![
            EarleySymbol::NonTerminal(MyEarleyNonTerm::Sentence),
            EarleySymbol::Terminal(end_term(end)),
        ],
    );

    let _res = earley_parse(str, grammar, MyEarleyNonTerm::StartSym);

    Ok(Sentence {})
}

pub fn parse_value(str: &mut Stream, end: &str, doc: &Document) -> ParseResult<Value> {
    let mut grammar = build_grammar(doc);
    grammar.add_rule(
        MyEarleyNonTerm::StartSym,
        vec![
            EarleySymbol::NonTerminal(MyEarleyNonTerm::Value),
            EarleySymbol::Terminal(end_term(end)),
        ],
    );

    let _res = earley_parse(str, grammar, MyEarleyNonTerm::StartSym);

    Ok(Value {})
}

pub fn parse_sentence_or_value(str: &mut Stream, end: &str, doc: &Document) -> ParseResult<Term> {
    let mut grammar = build_grammar(doc);
    grammar.add_rule(
        MyEarleyNonTerm::StartSym,
        vec![
            EarleySymbol::NonTerminal(MyEarleyNonTerm::Value),
            EarleySymbol::Terminal(end_term(end)),
        ],
    );
    grammar.add_rule(
        MyEarleyNonTerm::StartSym,
        vec![
            EarleySymbol::NonTerminal(MyEarleyNonTerm::Sentence),
            EarleySymbol::Terminal(end_term(end)),
        ],
    );

    let _res = earley_parse(str, grammar, MyEarleyNonTerm::StartSym);

    Ok(Term::Sentence(Sentence {}))
}
