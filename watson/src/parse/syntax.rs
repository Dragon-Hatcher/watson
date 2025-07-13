use crate::{
    parse::{
        Document,
        pattern::{PatternId, parse_pattern},
        stream::{ParseError, ParseResult, Stream},
    },
    statements::StatementId,
};

#[derive(Debug)]
pub struct Syntax {
    stmt_id: StatementId,
    patterns: Vec<PatternId>,
}

pub fn parse_syntax(str: &mut Stream, doc: &mut Document, stmt_id: StatementId) -> ParseResult<()> {
    str.commit(|str| {
        str.expect_str("syntax")?;

        let mut patterns = Vec::new();

        loop {
            match parse_pattern(str) {
                Ok(p) => patterns.push(p),
                Err(ParseError::Backtrack) => break,
                Err(ParseError::Commit) => return Err(ParseError::Commit),
            }
        }

        str.expect_str("end")?;

        let patterns = doc.patterns.patterns_for(stmt_id).to_owned();
        let syntax = Syntax { stmt_id, patterns };
        doc.syntax.push(syntax);

        Ok(())
    })
}
