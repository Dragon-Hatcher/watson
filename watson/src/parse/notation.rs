use crate::{
    parse::{find_patterns::PatternId, term::Sentence},
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Notation {
    statement: StatementId,
    name: Ustr,
    pattern: PatternId,
    replacement: Sentence,
}
