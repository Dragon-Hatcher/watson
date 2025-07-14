use super::pattern::PatternTy;
use crate::{
    parse::{
        common::{parse_kw, parse_name}, pattern::{parse_pattern, PatternId}, stream::{ParseResult, Stream}, term::{parse_sentence, Sentence}, Document
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
    str.commit(|str| {
        parse_kw(str, "notation")?;

        let name = parse_name(str)?;
        let _pattern = parse_pattern(str, PatternTy::Sentence)?;
        str.expect_str("=>")?;
        let replacement = parse_sentence(str, "end", doc)?;

        let notation = SentenceNotation {
            stmt_id,
            name,
            pattern: doc.patterns.patterns_for(stmt_id)[0],
            replacement,
        };
        doc.sentence_notations.push(notation);

        Ok(())
    })
}
