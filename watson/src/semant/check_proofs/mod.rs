use crate::{
    context::Ctx,
    diagnostics::WResult,
    semant::{
        check_proofs::lua_api::{LuaInfo, proof_to_lua::LuaProofState, setup_lua},
        proof_kernel::ProofState,
        proof_status::{ProofStatus, ProofStatuses},
        tactic::unresolved_proof::{TacticInst, UnresolvedProof},
        theorems::{_debug_theorem, TheoremId},
    },
};
use mlua::IntoLua;

mod lua_api;

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
            UnresolvedProof::Theorem(proof) => {
                let Ok(status) = check_theorem(*theorem, proof, &info, ctx) else {
                    continue;
                };
                status
            }
        };
        statuses.add(*theorem, status);
    }

    statuses
}

fn check_theorem<'ctx>(
    thm: TheoremId<'ctx>,
    tactic: &TacticInst<'ctx>,
    lua: &LuaInfo,
    ctx: &mut Ctx<'ctx>,
) -> WResult<ProofStatus<'ctx>> {
    println!("{}", _debug_theorem(thm));

    let proof_state =
        ProofState::new_from_theorem(thm, ctx).expect("theorem statement should be valid.");

    let lua_tactic: mlua::Value = tactic
        .into_lua(&lua.runtime)
        .or_else(|e| ctx.diags.err_lua_execution_error("tactic", e))?;
    let lua_proof_state: mlua::Value = LuaProofState::new(proof_state)
        .into_lua(&lua.runtime)
        .or_else(|e| ctx.diags.err_lua_execution_error("tactic", e))?;

    // Call the tactic handler
    let proof = lua
        .handle_tactic_fn
        .call::<LuaProofState>((lua_tactic, lua_proof_state))
        .or_else(|e| ctx.diags.err_lua_execution_error("tactic", e))?;
    let proof = proof.out::<'ctx>();
    let cert = proof.complete(ctx).expect("TODO");

    Ok(ProofStatus::from_cert(cert))
}
