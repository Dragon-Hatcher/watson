mod lua_api;

use mlua::IntoLua;

use crate::{
    context::Ctx,
    semant::{
        check_proofs::lua_api::{LuaInfo, setup_lua},
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

    let Ok(info) = setup_lua(ctx) else {
        // Errors have already been created so this is fine.
        return statuses;
    };

    for (theorem, proof) in theorems {
        let status = match proof {
            UnresolvedProof::Axiom => ProofStatus::new_axiom(),
            UnresolvedProof::Theorem(proof) => check_theorem(*theorem, proof, &info, ctx),
        };
        statuses.add(*theorem, status);
    }

    statuses
}

fn check_theorem<'ctx>(
    thm: TheoremId<'ctx>,
    proof: &TacticInst<'ctx>,
    lua: &LuaInfo,
    ctx: &mut Ctx<'ctx>,
) -> ProofStatus<'ctx> {
    println!();
    println!("Theorem {}", thm.name());

    let lua_tactic: mlua::Value = proof.into_lua(&lua.runtime).unwrap();
    dbg!(lua_tactic);

    todo!()
}
