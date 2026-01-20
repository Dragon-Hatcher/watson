use crate::semant::{
    check_proofs::lua_api::{
        frag_to_lua::{LuaPresFact, LuaPresFrag},
        notation_to_lua::LuaNotationBinding,
    },
    tactic::tactic_info::TacticInfo,
};
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, FromLua)]
pub struct LuaTacticInfo {
    info: TacticInfo<'static>,
}

impl LuaTacticInfo {
    pub fn new<'ctx>(info: TacticInfo<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let info: TacticInfo<'static> = unsafe { std::mem::transmute(info) };
        Self { info }
    }

    pub fn out_ref<'ctx>(&self) -> &TacticInfo<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(&self.info) }
    }
}

impl UserData for LuaTacticInfo {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("withAssume", |_, this, f: LuaPresFrag| {
            let new_info = this.out_ref().clone().with_assume(f.out());
            Ok(LuaTacticInfo::new(new_info))
        });

        methods.add_method("withDeduce", |_, this, f: LuaPresFact| {
            let new_info = this.out_ref().clone().with_deduce(f.out());
            Ok(LuaTacticInfo::new(new_info))
        });

        methods.add_method(
            "withLet",
            |_, this, (binding, replacement): (LuaNotationBinding, Option<LuaPresFrag>)| {
                let new_info = this
                    .out_ref()
                    .clone()
                    .with_let(binding.out(), replacement.map(|r| r.out()));
                Ok(LuaTacticInfo::new(new_info))
            },
        );

        methods.add_method("withGoal", |_, this, f: LuaPresFrag| {
            let new_info = this.out_ref().clone().with_goal(f.out());
            Ok(LuaTacticInfo::new(new_info))
        });
    }
}
