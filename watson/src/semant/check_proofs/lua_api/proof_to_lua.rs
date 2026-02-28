use crate::semant::{
    check_proofs::lua_api::{
        ctx_to_lua::LuaCtx, frag_to_lua::LuaPresFrag, theorem_to_lua::LuaTheorem,
    },
    proof_kernel::ProofState,
};
use itertools::Itertools;
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, FromLua)]
pub struct LuaProofState {
    proof: ProofState<'static>,
}

impl LuaProofState {
    pub fn new<'ctx>(proof: ProofState<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let proof: ProofState<'static> = unsafe { std::mem::transmute(proof) };
        Self { proof }
    }

    pub fn out<'ctx>(self) -> ProofState<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.proof) }
    }

    pub fn out_ref<'ctx>(&self) -> &ProofState<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(&self.proof) }
    }
}

impl UserData for LuaProofState {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("theorem", |_, this| {
            Ok(LuaTheorem::new(this.out_ref().theorem()))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("addAssumption", |lua, this, assumption: LuaPresFrag| {
            let assumption = assumption.out();
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();

            let new_state = this
                .out_ref()
                .add_assumption(assumption.frag(), ctx)
                .expect("TODO");
            Ok(LuaProofState::new(new_state))
        });

        methods.add_method("popAssumption", |lua, this, justifying: LuaPresFrag| {
            let justifying = justifying.out();
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();

            let new_state = this
                .out_ref()
                .pop_assumption(justifying.frag(), ctx)
                .expect("TODO");
            Ok(LuaProofState::new(new_state))
        });

        methods.add_method(
            "applyTheorem",
            |lua, this, (thm, templates): (LuaTheorem, Vec<LuaPresFrag>)| {
                let thm = thm.out();
                let templates = templates.into_iter().map(|t| t.out().frag()).collect_vec();
                let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();

                let new_state = this
                    .out_ref()
                    .apply_theorem(thm, &templates, ctx)
                    .expect("TODO");
                Ok(LuaProofState::new(new_state))
            },
        );

        methods.add_method(
            "applyTodo",
            |lua, this, (justifying, reason): (LuaPresFrag, Option<String>)| {
                let justifying = justifying.out();
                let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();

                let new_state = this
                    .out_ref()
                    .apply_todo(justifying.frag(), reason, ctx)
                    .expect("TODO");
                Ok(LuaProofState::new(new_state))
            },
        );

        methods.add_method("applyError", |lua, this, justifying: LuaPresFrag| {
            let justifying = justifying.out();
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();

            let new_state = this
                .out_ref()
                .apply_error(justifying.frag(), ctx)
                .expect("TODO");
            Ok(LuaProofState::new(new_state))
        });
    }
}
