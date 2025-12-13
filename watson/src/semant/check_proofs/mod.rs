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
    util::ansi::{ANSI_GRAY, ANSI_RESET},
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

/// Set to true to print logs from Lua scripts during theorem checking
const PRINT_LOGS: bool = true;

fn check_theorem<'ctx>(
    thm: TheoremId<'ctx>,
    proof: &TacticInst<'ctx>,
    lua: &LuaInfo,
    ctx: &mut Ctx<'ctx>,
) -> ProofStatus<'ctx> {
    let lua_tactic: mlua::Value = proof.into_lua(&lua.runtime).unwrap();

    // Call the tactic handler
    let thm_name = lua.runtime.create_string(thm.name().as_str()).unwrap();
    let _result: mlua::Value = lua.handle_tactic_fn.call((lua_tactic, thm_name)).unwrap();

    // Print logs if enabled
    if PRINT_LOGS {
        let logs = lua.get_logs();
        if !logs.is_empty() {
            println!("Logs while checking theorem {}:", thm.name());
            for log in logs {
                println!("{ANSI_GRAY}{log}{ANSI_RESET}");
            }
        }
    }

    lua.clear_logs();

    todo!()
}
