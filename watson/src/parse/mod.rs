use crate::{
    diagnostics::{ReportTracker, WResult, specifics},
    parse::{
        axiom::Axiom,
        definition::{Definition, DefinitionNotation},
        find_patterns::{PatternArena, find_patterns},
        notation::SentenceNotation,
        stream::Stream,
        syntax::Syntax,
        theorem::Theorem,
    },
    statements::{Statement, StatementTy, StatementsSet},
};

mod axiom;
mod common;
mod definition;
mod earley;
mod find_patterns;
mod hypotheses;
mod notation;
mod pattern;
mod proof;
pub mod stream;
mod syntax;
mod templates;
mod term;
mod theorem;

#[derive(Debug, Default)]
pub struct Document {
    patterns: PatternArena,

    // Which sentences are allowed?
    syntax: Vec<Syntax>,
    sentence_notations: Vec<SentenceNotation>,
    // Which values are allowed?
    definitions: Vec<Definition>,
    definition_notations: Vec<DefinitionNotation>,
    // Which deductions are allowed?
    axioms: Vec<Axiom>,
    theorems: Vec<Theorem>,
}

pub fn parse(ss: StatementsSet, tracker: &mut ReportTracker) -> WResult<Document> {
    let mut doc = Document::default();
    doc.patterns = find_patterns(&ss, tracker)?;

    for s in ss.statements() {
        parse_statement(s, &mut doc, tracker);
    }

    tracker.checkpoint()?;
    Ok(doc)
}

fn parse_statement(s: &Statement, doc: &mut Document, tracker: &mut ReportTracker) {
    let text = s.text().as_str();
    let mut str = Stream::new(text);

    let res = match s.ty() {
        StatementTy::Syntax => syntax::parse_syntax(&mut str, doc, s.id()),
        StatementTy::Notation => notation::parse_notation(&mut str, doc, s.id()),
        StatementTy::Definition => definition::parse_definition(&mut str, doc, s.id()),
        StatementTy::Axiom => axiom::parse_axiom(&mut str, doc, s.id()),
        StatementTy::Theorem => theorem::parse_theorem(&mut str, doc, s.id()),
        StatementTy::Prose => return,
    };

    let res = res.or_else(|_| str.expect_eof());

    if let Err(e) = res {
        tracker.add_message(specifics::parse_error(e, s.span()));
    }
}
