use crate::{
    diagnostics::{ReportTracker, WResult},
    parse::{
        axiom::Axiom,
        definition::{Definition, DefinitionNotation},
        find_patterns::{PatternArena, find_patterns},
        notation::Notation,
        syntax::Syntax,
        theorem::Theorem,
    },
    statements::{Statement, StatementsSet},
};

mod axiom;
mod common;
mod definition;
mod find_patterns;
mod hypotheses;
mod notation;
mod proof;
mod stream;
mod syntax;
mod templates;
mod term;
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
    let patterns = find_patterns(&ss, tracker)?;
    let mut doc = Document {
        patterns,
        syntax: Vec::new(),
        sentence_notations: Vec::new(),
        definitions: Vec::new(),
        definition_notations: Vec::new(),
        axioms: Vec::new(),
        theorems: Vec::new(),
    };

    for s in ss.statements() {
        parse_statement(s, &mut doc, tracker);
    }

    tracker.checkpoint()?;
    Ok(doc)
}

fn parse_statement(s: &Statement, doc: &mut Document, tracker: &mut ReportTracker) {
    todo!()
}
