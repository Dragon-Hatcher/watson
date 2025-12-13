use crate::{
    context::Ctx,
    semant::{
        proof_status::{ProofStatus, ProofStatuses},
        tactic::unresolved_proof::{TacticInst, UnresolvedProof},
        theorems::TheoremId,
    },
};

pub fn check_proofs<'ctx>(
    theorems: &[(TheoremId<'ctx>, UnresolvedProof<'ctx>)],
    ctx: &mut Ctx<'ctx>,
) -> ProofStatuses<'ctx> {
    let mut statuses = ProofStatuses::new();

    for (theorem, proof) in theorems {
        let status = match proof {
            UnresolvedProof::Axiom => ProofStatus::new_axiom(),
            UnresolvedProof::Theorem(proof) => check_theorem(*theorem, proof, ctx),
        };
        statuses.add(*theorem, status);
    }

    statuses
}

fn check_theorem<'ctx>(
    thm: TheoremId<'ctx>,
    proof: &TacticInst<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> ProofStatus<'ctx> {
    todo!()
}
