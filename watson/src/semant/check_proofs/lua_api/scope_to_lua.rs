use crate::semant::{
    check_proofs::lua_api::{
        formal_to_lua::LuaFormalCat, frag_to_lua::LuaPresFrag, notation_to_lua::LuaNotationBinding,
    },
    scope::{Scope, ScopeEntry},
};
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, FromLua)]
pub struct LuaScope {
    scope: Scope<'static>,
}

impl LuaScope {
    pub fn new<'ctx>(scope: Scope<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let scope: Scope<'static> = unsafe { std::mem::transmute(scope) };
        Self { scope }
    }

    pub fn out<'ctx>(self) -> Scope<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.scope) }
    }

    pub fn out_ref<'ctx>(&self) -> &Scope<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(&self.scope) }
    }
}

impl UserData for LuaScope {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "bindFrag",
            |_, this, (binding, frag): (LuaNotationBinding, LuaPresFrag)| {
                let binding = binding.out();
                let entry = ScopeEntry::new(frag.out());
                let new_scope = this.out_ref().child_with(binding, entry);
                Ok(LuaScope::new(new_scope))
            },
        );

        methods.add_method(
            "bindHole",
            |_, this, (binding, idx): (LuaNotationBinding, usize)| {
                let binding = binding.out();
                let entry = ScopeEntry::new_hole(binding.pattern().cat(), idx);
                let new_scope = this.out_ref().child_with(binding, entry);
                Ok(LuaScope::new(new_scope))
            },
        );
    }
}
