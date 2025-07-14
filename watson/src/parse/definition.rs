use super::pattern::PatternTy;
use crate::{
    parse::{
        Document,
        common::{parse_kw, parse_name},
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
    str.commit(|str| {
        parse_kw(str, "definition")?;

        let name = parse_name(str)?;
        let _pattern = parse_pattern(str, PatternTy::Value)?;
        let pattern = doc.patterns.patterns_for(stmt_id)[0];

        if str.expect_str("=>").is_ok() {
            let replacement = parse_value(str, "end", doc)?;

            let notation = DefinitionNotation {
                stmt_id,
                name,
                pattern,
                replacement,
            };
            doc.definition_notations.push(notation);
        } else {
            parse_kw(str, "where")?;
            let hypotheses = parse_hypotheses(str, doc)?;
            str.expect_str("|-")?;
            let conclusion = parse_sentence(str, "proof", doc)?;
            let proof = parse_proof(str, doc)?;
            parse_kw(str, "end")?;

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
