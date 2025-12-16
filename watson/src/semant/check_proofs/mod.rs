use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, WResult},
    semant::{
        check_proofs::lua_api::{LuaInfo, proof_to_lua::LuaProofState, setup_lua},
        proof_kernel::ProofState,
        proof_status::{ProofStatus, ProofStatuses},
        tactic::unresolved_proof::{TacticInst, UnresolvedProof},
        theorems::TheoremId,
    },
};
use mlua::IntoLua;

mod lua_api;

pub fn check_proofs<'ctx>(
    theorems: &[(TheoremId<'ctx>, UnresolvedProof<'ctx>)],
    ctx: &mut Ctx<'ctx>,
) -> ProofStatuses<'ctx> {
    let mut statuses = ProofStatuses::new();

    let info = match setup_lua(ctx) {
        Ok(info) => info,
        Err(diags) => {
            // Failed to set up Lua. Add the diagnostics and return.
            ctx.diags.add_diags(diags);
            return statuses;
        }
    };

    for (theorem, proof) in theorems {
        let status = match proof {
            UnresolvedProof::Axiom => ProofStatus::new_axiom(),
            UnresolvedProof::Theorem(proof) => {
                match check_theorem(*theorem, proof, &info, ctx) {
                    Ok(status) => status,
                    Err(diags) => {
                        // Error checking theorem. Add the diagnostics and continue.
                        ctx.diags.add_diags(diags);
                        continue;
                    }
                }
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
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, ProofStatus<'ctx>> {
    let proof_state =
        ProofState::new_from_theorem(thm, ctx).expect("theorem statement should be valid.");

    let lua_tactic: mlua::Value = tactic
        .into_lua(&lua.runtime)
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;
    let lua_proof_state: mlua::Value = LuaProofState::new(proof_state)
        .into_lua(&lua.runtime)
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;

    // Call the tactic handler
    let proof = lua
        .handle_tactic_fn
        .call::<LuaProofState>((lua_tactic, lua_proof_state))
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;
    let proof = proof.out::<'ctx>();
    let cert = proof.complete(ctx).expect("TODO");

    Ok(ProofStatus::from_cert(cert))
}
