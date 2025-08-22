use crate::semant::{
    formal_syntax::FormalSyntax,
    fragments::{FragCtx, resolve_frag},
    theorem::{Fact, TheoremId, TheoremStatement, TheoremStatements},
    unresolved::{UnresolvedFragment, UnresolvedTheorem},
};
use std::collections::HashMap;

pub mod formal_syntax;
mod fragments;
pub mod theorem;
pub mod unresolved;

pub fn check_proofs(theorems: HashMap<TheoremId, UnresolvedTheorem>, formal: &FormalSyntax) {
    let mut ctx = FragCtx::new();
    let statements = collect_theorem_statements(&theorems, formal, &mut ctx);
    dbg!(statements);
}

fn collect_theorem_statements(
    theorems: &HashMap<TheoremId, UnresolvedTheorem>,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
) -> TheoremStatements {
    let mut statements = TheoremStatements::new();
    for theorem in theorems.values() {
        let statement = theorem_statement_from_unresolved(theorem.clone(), formal, ctx);
        statements.add(statement);
    }
    statements
}

fn theorem_statement_from_unresolved(
    unresolved: UnresolvedTheorem,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
) -> TheoremStatement {
    let mut templates = HashMap::new();
    let shorthands = HashMap::new();
    let mut bindings = Vec::new();

    for template in &unresolved.templates {
        templates.insert(template.name(), template);
    }

    let mut resolve = |u_frag: UnresolvedFragment| {
        resolve_frag(u_frag, &templates, &shorthands, &mut bindings, formal, ctx)
    };

    TheoremStatement::new(
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
    )
}
