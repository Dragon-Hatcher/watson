use crate::{
    parse::{
        find_patterns::PatternId,
        proof::Proof,
        term::{Sentence, Value},
    },
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Definition {
    statement: StatementId,
    name: Ustr,
    pattern: PatternId,
    conclusion: Sentence,
    proof: Proof,
}

#[derive(Debug)]
pub struct DefinitionNotation {
    statement: StatementId,
    name: Ustr,
    pattern: PatternId,
    replacement: Value,
}
