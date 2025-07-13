use crate::{
    parse::{
        Document, Proof, Sentence,
        common::parse_name,
        hypotheses::parse_hypotheses,
        proof::parse_proof,
        sentence::parse_sentence,
        stream::{ParseResult, Stream},
        templates::{Template, parse_templates},
    },
    statements::StatementId,
};
use ustr::Ustr;

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
    str.expect_str("axiom")?;
    let name = parse_name(str)?;
    let templates = parse_templates(str)?;
    str.expect_char(':')?;
    let hypotheses = parse_hypotheses(str)?;
    str.expect_str("|-")?;
    let conclusion = parse_sentence(str)?;
    str.expect_str("proof")?;
    let proof = parse_proof(str)?;
    str.expect_str("qed")?;

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
}
