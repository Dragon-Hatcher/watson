use crate::{
    parse::{
        Document,
        common::parse_name,
        hypotheses::parse_hypotheses,
        stream::{ParseResult, Stream},
        templates::{Template, parse_templates},
        term::{Sentence, parse_sentence},
    },
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Axiom {
    stmt_id: StatementId,
    name: Ustr,
    templates: Vec<Template>,
    hypotheses: Vec<Sentence>,
    conclusion: Sentence,
}

pub fn parse_axiom(str: &mut Stream, doc: &mut Document, stmt_id: StatementId) -> ParseResult<()> {
    str.expect_str("axiom")?;
    let name = parse_name(str)?;
    let templates = parse_templates(str)?;
    str.expect_char(':')?;
    let hypotheses = parse_hypotheses(str)?;
    str.expect_str("|-")?;
    let conclusion = parse_sentence(str)?;
    str.expect_str("end")?;

    let axiom = Axiom {
        stmt_id,
        name,
        templates,
        hypotheses,
        conclusion,
    };
    doc.axioms.push(axiom);

    Ok(())
}
