use crate::parse::{
    stream::{ParseError, ParseResult, Stream},
    term::{Sentence, parse_sentence},
};

pub fn parse_hypotheses(str: &mut Stream) -> ParseResult<Vec<Sentence>> {
    let mut hypotheses = Vec::new();

    loop {
        match parse_hypothesis(str) {
            Ok(s) => hypotheses.push(s),
            Err(ParseError::Backtrack) => break,
            Err(ParseError::Commit) => return Err(ParseError::Commit),
        }
    }

    Ok(hypotheses)
}

fn parse_hypothesis(str: &mut Stream) -> ParseResult<Sentence> {
    str.expect_char('(')?;

    str.commit(|str| {
        let sentence = parse_sentence(str)?;
        str.expect_char(')')?;
        Ok(sentence)
    })
}
