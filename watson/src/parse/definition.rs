use crate::{
    parse::{
        Document,
        common::parse_name,
        hypotheses::parse_hypotheses,
        pattern::{PatternId, parse_pattern},
        proof::{Proof, parse_proof},
        stream::{ParseResult, Stream},
        term::{Sentence, Value, parse_sentence, parse_value},
    },
    statements::StatementId,
};
use ustr::Ustr;

#[derive(Debug)]
pub struct Definition {
    stmt_id: StatementId,
    name: Ustr,
    pattern: PatternId,
    hypotheses: Vec<Sentence>,
    conclusion: Sentence,
    proof: Proof,
}

#[derive(Debug)]
pub struct DefinitionNotation {
    stmt_id: StatementId,
    name: Ustr,
    pattern: PatternId,
    replacement: Value,
}

pub fn parse_definition(
    str: &mut Stream,
    doc: &mut Document,
    stmt_id: StatementId,
) -> ParseResult<()> {
    str.expect_str("definition")?;

    str.commit(|str| {
        let name = parse_name(str)?;
        let pattern = parse_pattern(str, &mut doc.patterns)?;

        if str.expect_str("=>").is_ok() {
            let replacement = parse_value(str)?;
            str.expect_str("end")?;

            let notation = DefinitionNotation {
                stmt_id,
                name,
                pattern,
                replacement,
            };
            doc.definition_notations.push(notation);
        } else {
            str.expect_str("where")?;
            let hypotheses = parse_hypotheses(str)?;
            str.expect_str("|-")?;
            let conclusion = parse_sentence(str)?;
            str.expect_str("proof")?;
            let proof = parse_proof(str)?;
            str.expect_str("end")?;

            let def = Definition {
                stmt_id,
                name,
                pattern,
                hypotheses,
                conclusion,
                proof,
            };
            doc.definitions.push(def);
        }

        Ok(())
    })
}
