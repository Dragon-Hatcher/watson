use crate::parse::{
    Document,
    stream::{ParseError, ParseResult, Stream},
    term::{Sentence, parse_sentence},
};

pub fn parse_hypotheses(str: &mut Stream, doc: &Document) -> ParseResult<Vec<Sentence>> {
    let mut hypotheses = Vec::new();

    loop {
        match parse_hypothesis(str, doc) {
            Ok(s) => hypotheses.push(s),
            Err(ParseError::Backtrack(_)) => break,
            Err(ParseError::Commit(e)) => return Err(ParseError::Commit(e)),
        }
    }

    Ok(hypotheses)
}

fn parse_hypothesis(str: &mut Stream, doc: &Document) -> ParseResult<Sentence> {
    str.expect_char('(')?;

    str.commit(|str| {
        let sentence = parse_sentence(str, ")", doc)?;
        Ok(sentence)
    })
}
