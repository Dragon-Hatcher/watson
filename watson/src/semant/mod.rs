use crate::{
    diagnostics::DiagManager,
    parse::macros::Macros,
    semant::{
        formal_syntax::FormalSyntax,
        fragments::{FragCtx, resolve_frag},
        theorem::{Fact, TheoremId, TheoremStatement, TheoremStatements},
        unresolved::{UnresolvedFragment, UnresolvedProof, UnresolvedTheorem},
    },
};
use std::collections::HashMap;

mod check_proofs;
pub mod formal_syntax;
mod fragments;
pub mod theorem;
pub mod unresolved;

pub fn check_proofs(
    theorems: HashMap<TheoremId, UnresolvedTheorem>,
    formal: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) {
    let mut ctx = FragCtx::new();
    let (statements, proofs) = collect_theorem_statements(&theorems, formal, &mut ctx);
    check_proofs::check_proofs(&statements, proofs, formal, macros, diags, &mut ctx);
}

fn collect_theorem_statements(
    theorems: &HashMap<TheoremId, UnresolvedTheorem>,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
) -> (TheoremStatements, HashMap<TheoremId, UnresolvedProof>) {
    let mut statements = TheoremStatements::new();
    let mut proofs = HashMap::new();
    for theorem in theorems.values() {
        let (statement, proof) = theorem_statement_from_unresolved(theorem.clone(), formal, ctx);
        statements.add(statement);
        proofs.insert(theorem.id, proof);
    }
    (statements, proofs)
}

fn theorem_statement_from_unresolved(
    unresolved: UnresolvedTheorem,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
) -> (TheoremStatement, UnresolvedProof) {
    let mut templates = HashMap::new();
    let shorthands = HashMap::new();
    let mut bindings = Vec::new();

    for template in &unresolved.templates {
        templates.insert(template.name(), template);
    }

    let mut resolve = |u_frag: UnresolvedFragment| {
        resolve_frag(
            u_frag,
            &templates,
            &shorthands,
            &mut bindings,
            false,
            formal,
            ctx,
        )
    };

    let statement = TheoremStatement::new(
        unresolved.id,
        unresolved.templates.clone(),
        unresolved
            .hypotheses
            .into_iter()
            .map(|u_fact| {
                Fact::new(
                    u_fact.assumption.map(&mut resolve),
                    resolve(u_fact.statement),
                )
            })
            .collect(),
        resolve(unresolved.conclusion),
    );
    (statement, unresolved.proof)
}
