use crate::semant::{check_proofs::lua_api::ctx_to_lua::LuaCtx, formal_syntax::FormalSyntaxCatId};
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, FromLua)]
pub struct LuaFormalCat {
    cat: FormalSyntaxCatId<'static>,
}

impl LuaFormalCat {
    pub fn new<'ctx>(cat: FormalSyntaxCatId<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let cat: FormalSyntaxCatId<'static> = unsafe { std::mem::transmute(cat) };
        Self { cat }
    }

    pub fn out<'ctx>(&self) -> FormalSyntaxCatId<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.cat) }
    }
}

impl UserData for LuaFormalCat {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.out().name().to_string()));
    }
}

pub struct LuaFormalCatMeta;

impl UserData for LuaFormalCatMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("sentence", |lua, _, _: ()| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            Ok(LuaFormalCat::new(ctx.sentence_cat))
        });
    }
}
