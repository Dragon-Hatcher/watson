use crate::{
    parse::{
        common::{parse_kw, parse_name}, hypotheses::parse_hypotheses, proof::{parse_proof, Proof}, stream::{ParseResult, Stream}, templates::{parse_templates, Template}, term::{parse_sentence, Sentence}, Document
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
        let proof = parse_proof(str)?;
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
