use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, DiagnosticSpan, WResult},
    parse::Span,
    semant::{
        check_proofs::lua_api::{
            LuaInfo, diag_to_lua::LuaDiagnostic, proof_to_lua::LuaProofState, setup_lua,
            tactic_info_to_lua::LuaTacticInfo, theorem_to_lua::LuaTheorem,
        },
        proof_kernel::ProofState,
        proof_status::{ProofStatus, ProofStatuses},
        tactic::{
            tactic_info::TacticInfo,
            unresolved_proof::{TacticInst, UnresolvedProof},
        },
        theorems::TheoremId,
    },
};
use mlua::IntoLua;
use std::{cell::RefCell, rc::Rc, vec};
use ustr::Ustr;

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

impl<'ctx> Diagnostic<'ctx> {
    pub fn err_tactic_did_not_prove<T>(thm: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("tactic for theorem `{thm}` did not prove goal"),
            vec![DiagnosticSpan::new_error("", span)],
        );
        Err(vec![diag])
    }
}

struct LuaTheoremInfoInner {
    thm: LuaTheorem,
    has_logs: bool,
    diags: Vec<LuaDiagnostic>,
}
type LuaTheoremInfo = Rc<RefCell<LuaTheoremInfoInner>>;

fn check_theorem<'ctx>(
    thm: TheoremId<'ctx>,
    tactic: &TacticInst<'ctx>,
    lua: &LuaInfo,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, ProofStatus<'ctx>> {
    let proof_state =
        ProofState::new_from_theorem(thm, ctx).expect("theorem statement should be valid.");

    let lua_tactic: mlua::Value = tactic
        .into_lua(&lua.runtime)
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;
    let lua_proof_state: mlua::Value = LuaProofState::new(proof_state)
        .into_lua(&lua.runtime)
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;
    let lua_tactic_info: mlua::Value = LuaTacticInfo::new(TacticInfo::new(thm))
        .into_lua(&lua.runtime)
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;

    let theorem_info = LuaTheoremInfoInner {
        thm: LuaTheorem::new(thm),
        has_logs: false,
        diags: Vec::new(),
    };
    let theorem_info = Rc::new(RefCell::new(theorem_info));
    lua.runtime.set_app_data(theorem_info.clone());

    // Call the tactic handler
    let proof = lua
        .handle_tactic_fn
        .call::<LuaProofState>((lua_tactic, lua_proof_state, lua_tactic_info))
        .or_else(|e| Diagnostic::err_lua_execution_error("tactic", e))?;
    let proof = proof.out::<'ctx>();
    let cert = proof
        .complete(ctx)
        .or_else(|_| Diagnostic::err_tactic_did_not_prove(thm.name(), tactic.span()))?;

    // Add diagnostics reported by the tactic.
    for diag in theorem_info.borrow_mut().diags.drain(..) {
        ctx.diags.add_diag(diag.out());
    }

    // Add blank space after logs.
    if theorem_info.borrow().has_logs {
        eprintln!();
    }

    Ok(ProofStatus::from_cert(cert))
}
