use crate::semant::{
    check_proofs::lua_api::{
        ctx_to_lua::LuaCtx,
        formal_to_lua::LuaFormalCat,
        frag_to_lua::{LuaPresFact, LuaPresFrag},
        notation_to_lua::LuaNotationBinding,
        scope_to_lua::LuaScope,
    },
    notation::NotationBinding,
    theorems::{Template, TheoremId, TheoremStatement},
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

        fields.add_field_method_get("templates", |_, this| {
            let vec = this
                .out()
                .templates()
                .iter()
                .map(|t| LuaTemplate::new(t.clone()))
                .collect_vec();
            Ok(vec)
        });

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

        fields.add_field_method_get("scope", |lua, this| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let scope = ctx.scopes._get(this.out().scope());
            Ok(LuaScope::new(scope))
        });
    }
}

pub struct LuaTheoremMeta;

impl UserData for LuaTheoremMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("lookupByName", |lua, _, name: String| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let thm = ctx.arenas.theorem_stmts.get(name.into());
            Ok(thm.map(|t| LuaTheorem::new(t)))
        });
    }
}

#[derive(Debug, Clone, FromLua)]
pub struct LuaTemplate {
    tmp: Template<'static>,
}

impl LuaTemplate {
    pub fn new<'ctx>(tmp: Template<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let cat: Template<'static> = unsafe { std::mem::transmute(tmp) };
        Self { tmp: cat }
    }

    pub fn out_ref<'ctx>(&self) -> &Template<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(&self.tmp) }
    }
}

impl UserData for LuaTemplate {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("cat", |_, this| {
            let cat = this.out_ref().cat();
            Ok(LuaFormalCat::new(cat))
        });

        fields.add_field_method_get("binding", |_, this| {
            let binding = this.out_ref().binding();
            Ok(LuaNotationBinding::new(binding))
        });

        fields.add_field_method_get("holes", |lua, this| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let bindings = this
                .out_ref()
                .holes()
                .iter()
                .map(|(cat, name)| {
                    let pattern = ctx.single_name_notations[cat];
                    let binding = NotationBinding::new(pattern, vec![*name]);
                    let binding = ctx.arenas.notation_bindings.intern(binding);
                    LuaNotationBinding::new(binding)
                })
                .collect_vec();
            Ok(bindings)
        });
    }
}
