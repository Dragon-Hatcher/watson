use crate::{
    parse::{
        Document,
        common::{parse_kw, parse_name},
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
    str.commit(|str| {
        parse_kw(str, "axiom")?;

        let name = parse_name(str)?;
        let templates = parse_templates(str)?;
        str.expect_char(':')?;
        let hypotheses = parse_hypotheses(str, doc)?;
        str.expect_str("|-")?;
        let conclusion = parse_sentence(str, "end", doc)?;

        let axiom = Axiom {
            stmt_id,
            name,
            templates,
            hypotheses,
            conclusion,
        };
        doc.axioms.push(axiom);

        Ok(())
    })
}
