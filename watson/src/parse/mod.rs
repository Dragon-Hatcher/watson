use crate::{
    diagnostics::{ReportTracker, WResult},
    parse::{axiom::Axiom, find_patterns::PatternId, theorem::Theorem},
    statements::{StatementId, StatementsSet},
};
use ustr::Ustr;

mod axiom;
mod common;
mod find_patterns;
mod hypotheses;
mod proof;
mod sentence;
mod stream;
mod templates;
mod theorem;

#[derive(Debug)]
pub struct Document {
    patterns: PatternArena,

    // Which sentences are allowed?
    syntax: Vec<Syntax>,
    sentence_notations: Vec<Notation>,
    // Which values are allowed?
    definitions: Vec<Definition>,
    definition_notations: Vec<DefinitionNotation>,
    // Which deductions are allowed?
    axioms: Vec<Axiom>,
    theorems: Vec<Theorem>,
}

pub fn parse(ss: StatementsSet, tracker: &mut ReportTracker) -> WResult<Document> {
    Err(())
}

pub struct Syntax {
    patterns: Vec<PatternId>,
}

pub struct Notation {
    statement: StatementId,
    name: Ustr,
    pattern: PatternId,
    replacement: Sentence,
}

pub struct Definition {
    statement: StatementId,
    name: Ustr,
    pattern: PatternId,
    conclusion: Sentence,
    proof: Proof,
}

pub struct DefinitionNotation {
    statement: StatementId,
    name: Ustr,
    pattern: PatternId,
    replacement: Value,
}

pub struct Sentence {
    pattern: PatternId,
    terms: Vec<Term>,
}

pub struct Value {
    pattern: PatternId,
    terms: Vec<Term>,
}

pub enum Term {
    Sentence(Sentence),
    Value(Value),
    Binding(Ustr),
}

pub struct Proof {
    tactics: Vec<Tactic>,
}

pub enum Tactic {
    Have(HaveTactic),
}

pub struct HaveTactic {
    conclusion: Sentence,
    by: Ustr,
    substitutions: Vec<Substitution>,
}

pub enum Substitution {
    Value(Value),
    Sentence(Sentence),
}
