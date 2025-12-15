use crate::semant::{
    check_proofs::lua_api::frag_to_lua::{LuaPresFact, LuaPresFrag},
    theorems::{TheoremId, TheoremStatement},
};
use itertools::Itertools;
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaTheorem {
    ptr: *const TheoremStatement<'static>,
}

impl LuaTheorem {
    pub fn new<'ctx>(thm: TheoremId<'ctx>) -> Self {
        let ptr = std::ptr::from_ref(thm.0);
        // SAFETY: We don't use this ptr here so this isn't really unsafe. See
        // where we dereference for the actual safety details.
        let ptr: *const TheoremStatement<'static> = unsafe { std::mem::transmute(ptr) };

        Self { ptr }
    }

    pub fn out<'ctx>(&self) -> TheoremId<'ctx> {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let parse_tree = unsafe { &*self.ptr };
        TheoremId(parse_tree)
    }
}

impl UserData for LuaTheorem {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.out().name().to_string()));

        fields.add_field_method_get("hypotheses", |_, this| {
            let vec = this
                .out()
                .hypotheses()
                .iter()
                .map(|pf| LuaPresFact::new(*pf))
                .collect_vec();
            Ok(vec)
        });

        fields.add_field_method_get("conclusion", |_, this| {
            Ok(LuaPresFrag::new(this.out().conclusion()))
        });
    }
}
