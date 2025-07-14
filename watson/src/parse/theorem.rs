use crate::{
    parse::{
        Document,
        common::{parse_kw, parse_name},
        hypotheses::parse_hypotheses,
        proof::{Proof, parse_proof},
        stream::{ParseResult, Stream},
        templates::{Template, parse_templates},
        term::{Sentence, parse_sentence},
    },
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Theorem {
    stmt_id: StatementId,
    name: Ustr,
    templates: Vec<Template>,
    hypotheses: Vec<Sentence>,
    conclusion: Sentence,
    proof: Proof,
}

pub fn parse_theorem(
    str: &mut Stream,
    doc: &mut Document,
    stmt_id: StatementId,
) -> ParseResult<()> {
    str.commit(|str| {
        parse_kw(str, "theorem")?;
        let name = parse_name(str)?;
        let templates = parse_templates(str)?;
        str.expect_char(':')?;
        let hypotheses = parse_hypotheses(str, doc)?;
        str.expect_str("|-")?;
        let conclusion = parse_sentence(str, "proof", doc)?;
        let proof = parse_proof(str, doc)?;
        parse_kw(str, "qed")?;

        let theorem = Theorem {
            stmt_id,
            name,
            templates,
            hypotheses,
            conclusion,
            proof,
        };
        doc.theorems.push(theorem);

        Ok(())
    })
}
