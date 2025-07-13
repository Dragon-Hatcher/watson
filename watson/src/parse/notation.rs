use crate::{
    parse::{
        Document,
        common::parse_name,
        pattern::{PatternId, parse_pattern},
        stream::{ParseResult, Stream},
        term::{Sentence, parse_sentence},
    },
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug)]
pub struct SentenceNotation {
    stmt_id: StatementId,
    name: Ustr,
    pattern: PatternId,
    replacement: Sentence,
}

pub fn parse_notation(
    str: &mut Stream,
    doc: &mut Document,
    stmt_id: StatementId,
) -> ParseResult<()> {
    str.expect_str("notation")?;

    str.commit(|str| {
        let name = parse_name(str)?;
        let pattern = parse_pattern(str, &mut doc.patterns)?;
        str.expect_str("=>")?;
        let replacement = parse_sentence(str)?;
        str.expect_str("end")?;

        let notation = SentenceNotation {
            stmt_id,
            name,
            pattern,
            replacement,
        };
        doc.sentence_notations.push(notation);

        Ok(())
    })
}
