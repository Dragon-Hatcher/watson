use crate::semant::{
    check_proofs::lua_api::{ctx_to_lua::LuaCtx, formal_to_lua::LuaFormalCat},
    notation::{NotationBinding, NotationBindingId},
};
use itertools::Itertools;
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaNotationBinding {
    binding: NotationBindingId<'static>,
}

impl LuaNotationBinding {
    pub fn new<'ctx>(binding: NotationBindingId<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let binding: NotationBindingId<'static> = unsafe { std::mem::transmute(binding) };
        Self { binding }
    }

    pub fn out<'ctx>(self) -> NotationBindingId<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.binding) }
    }
}

impl UserData for LuaNotationBinding {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("cat", |_, this| {
            let cat = this.out().pattern().cat();
            Ok(LuaFormalCat::new(cat))
        });

        fields.add_field_method_get("names", |_, this| {
            let names = this
                .out()
                .name_instantiations()
                .iter()
                .map(|s| s.to_string())
                .collect_vec();
            Ok(names)
        });
    }
}

pub struct LuaNotationBindingMeta;

impl UserData for LuaNotationBindingMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("name", |lua, _, (name, cat): (String, LuaFormalCat)| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let pattern = ctx.single_name_notations[&cat.out()];
            let binding = NotationBinding::new(pattern, vec![name.into()]);
            let binding = ctx.arenas.notation_bindings.intern(binding);
            Ok(LuaNotationBinding::new(binding))
        });
    }
}
