use crate::{
    diagnostics::{DiagManager, WResult},
    parse::macros::Macros,
    semant::{
        // check_proofs::ProofStatus,
        formal_syntax::FormalSyntax,
        // fragments::{FragCtx, resolve_frag},
        // theorem::{Fact, TheoremId, TheoremStatement, TheoremStatements},
        // unresolved::{UnresolvedProof, UnresolvedTheorem},
    },
};
use std::collections::HashMap;

// mod check_circularity;
// pub mod check_proofs;
pub mod formal_syntax;
// pub mod fragments;
// pub mod render_proof_state;
// pub mod theorem;
// pub mod unresolved;

// pub fn check_proofs(
//     theorems: HashMap<TheoremId, UnresolvedTheorem>,
//     formal: &FormalSyntax,
//     macros: &Macros,
//     diags: &mut DiagManager,
//     frag_ctx: &mut FragCtx,
// ) -> WResult<(ProofReport, TheoremStatements)> {
//     let (statements, proofs) = collect_theorem_statements(&theorems, formal, frag_ctx, diags)?;
//     let proof_statuses =
//         check_proofs::check_proofs(&statements, proofs, formal, macros, diags, frag_ctx);
//     let circular_groups = check_circularity::find_circular_dependency_groups(&proof_statuses);

//     let report = ProofReport {
//         statuses: proof_statuses,
//         circular_groups,
//     };
//     Ok((report, statements))
// }

// pub struct ProofReport {
//     pub statuses: HashMap<TheoremId, ProofStatus>,
//     pub circular_groups: Vec<Vec<TheoremId>>,
// }

// fn collect_theorem_statements(
//     theorems: &HashMap<TheoremId, UnresolvedTheorem>,
//     formal: &FormalSyntax,
//     ctx: &mut FragCtx,
//     diags: &mut DiagManager,
// ) -> WResult<(TheoremStatements, HashMap<TheoremId, UnresolvedProof>)> {
//     let mut statements = TheoremStatements::new();
//     let mut proofs = HashMap::new();
//     for theorem in theorems.values() {
//         let (statement, proof) =
//             theorem_statement_from_unresolved(theorem.clone(), formal, ctx, diags)?;
//         statements.add(statement);
//         proofs.insert(theorem.id, proof);
//     }
//     Ok((statements, proofs))
// }

// fn theorem_statement_from_unresolved(
//     unresolved: UnresolvedTheorem,
//     formal: &FormalSyntax,
//     ctx: &mut FragCtx,
//     diags: &mut DiagManager,
// ) -> WResult<(TheoremStatement, UnresolvedProof)> {
//     let mut templates = HashMap::new();
//     let shorthands = HashMap::new();
//     let mut bindings = Vec::new();

//     for template in &unresolved.templates {
//         templates.insert(template.name(), template);
//     }

//     let conclusion = resolve_frag(
//         unresolved.conclusion,
//         &templates,
//         &shorthands,
//         &mut bindings,
//         false,
//         formal,
//         ctx,
//         diags,
//         unresolved.id,
//         None,
//     );

//     let mut hypotheses = Vec::new();
//     for hypothesis in unresolved.hypotheses {
//         let assumption = hypothesis
//             .assumption
//             .map(|u_frag| {
//                 resolve_frag(
//                     u_frag,
//                     &templates,
//                     &shorthands,
//                     &mut bindings,
//                     false,
//                     formal,
//                     ctx,
//                     diags,
//                     unresolved.id,
//                     None,
//                 )
//             })
//             .transpose()?;
//         let statement = resolve_frag(
//             hypothesis.statement,
//             &templates,
//             &shorthands,
//             &mut bindings,
//             false,
//             formal,
//             ctx,
//             diags,
//             unresolved.id,
//             None,
//         )?;
//         hypotheses.push(Fact::new(assumption, statement));
//     }

//     let statement = TheoremStatement::new(
//         unresolved.id,
//         unresolved.templates.clone(),
//         hypotheses,
//         conclusion?,
//     );
//     Ok((statement, unresolved.proof))
// }
