use crate::semant::{
    check_proofs::lua_api::{ctx_to_lua::LuaCtx, formal_to_lua::LuaFormalCat},
    fragment::{_debug_fragment, Fragment, FragmentId, hole_frag, var_frag},
    presentation::{
        BindingNameHints, Pres, PresFrag, PresId, change_name_hints, instantiate_holes,
        instantiate_templates, instantiate_vars, match_presentation, wrap_frag_with_name,
    },
    theorems::PresFact,
};
use mlua::{FromLua, MetaMethod, UserData};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromLua)]
pub struct LuaPresFrag {
    frag: LuaFrag,
    pres: LuaPres,
    formal: LuaPres,
}

impl LuaPresFrag {
    pub fn new<'ctx>(frag: PresFrag<'ctx>) -> Self {
        Self {
            frag: LuaFrag::new(frag.frag()),
            pres: LuaPres::new(frag.pres()),
            formal: LuaPres::new(frag.formal_pres()),
        }
    }

    pub fn out<'ctx>(&self) -> PresFrag<'ctx> {
        PresFrag::new(self.frag.out(), self.pres.out(), self.formal.out())
    }
}

impl UserData for LuaPresFrag {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("cat", |_, this| {
            let cat = this.out().frag().cat();
            Ok(LuaFormalCat::new(cat))
        });

        fields.add_field_method_get("formal", |_, this| {
            Ok(LuaPresFrag::new(this.out().formal()))
        });

        fields.add_field_method_get("debug", |_, this| Ok(_debug_fragment(this.out().frag())));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("fact", |_, this, _: ()| {
            let fact = PresFact::new(None, this.out());
            Ok(LuaPresFact::new(fact))
        });

        methods.add_method("named", |lua, this, name: String| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let frag = wrap_frag_with_name(this.out(), name.into(), ctx);
            Ok(LuaPresFrag::new(frag))
        });

        methods.add_method("changeBinderNames", |lua, this, names: Vec<String>| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let names = names.iter().map(|n| n.into()).collect();
            let binding_names = BindingNameHints::new(names);
            let binding_names = ctx.arenas.binding_name_hints.intern(binding_names);
            let frag = change_name_hints(this.out(), binding_names, ctx);
            Ok(LuaPresFrag::new(frag))
        });

        methods.add_method(
            "instantiateTemplates",
            |lua, this, templates: Vec<LuaPresFrag>| {
                let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
                let frag = instantiate_templates(this.out(), &|idx| templates[idx].out(), ctx);
                Ok(LuaPresFrag::new(frag))
            },
        );

        methods.add_method("instantiateVars", |lua, this, vars: Vec<LuaPresFrag>| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let frag = instantiate_vars(this.out(), &|idx| vars[idx].out(), vars.len(), ctx);
            Ok(LuaPresFrag::new(frag))
        });

        methods.add_method("instantiateHoles", |lua, this, holes: Vec<LuaPresFrag>| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            // TODO: make shifting an option?
            let frag = instantiate_holes(this.out(), &|idx| holes[idx].out(), 0, false, ctx);
            Ok(LuaPresFrag::new(frag))
        });

        methods.add_method(
            "match",
            |_, this, pattern: LuaPresFrag| match match_presentation(this.out(), pattern.out()) {
                Some(matches) => {
                    let map = matches
                        .into_iter()
                        .map(|(idx, f)| (idx, LuaPresFrag::new(f)))
                        .collect::<FxHashMap<usize, LuaPresFrag>>();
                    Ok(Some(map))
                }
                None => Ok(None),
            },
        );

        methods.add_meta_method(MetaMethod::ToString, |_, this, _args: ()| {
            Ok(this.out().print())
        });

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Self| {
            Ok(this.out() == other.out())
        });
    }
}

pub struct LuaPresFragMeta;

impl UserData for LuaPresFragMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("var", |lua, _, (cat, idx): (LuaFormalCat, usize)| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let frag = var_frag(idx, cat.out(), ctx);
            Ok(LuaPresFrag::new(frag))
        });

        methods.add_method("hole", |lua, _, (cat, idx): (LuaFormalCat, usize)| {
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let frag = hole_frag(idx, cat.out(), Vec::new(), ctx);
            Ok(LuaPresFrag::new(frag))
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromLua)]
pub struct LuaPresFact {
    assumption: Option<LuaPresFrag>,
    conclusion: LuaPresFrag,
}

impl LuaPresFact {
    pub fn new<'ctx>(fact: PresFact<'ctx>) -> Self {
        Self {
            assumption: fact.assumption().map(LuaPresFrag::new),
            conclusion: LuaPresFrag::new(fact.conclusion()),
        }
    }

    pub fn out<'ctx>(&self) -> PresFact<'ctx> {
        PresFact::new(self.assumption.map(|f| f.out()), self.conclusion.out())
    }
}

impl UserData for LuaPresFact {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("assumption", |_, this| Ok(this.assumption));

        fields.add_field_method_get("conclusion", |_, this| Ok(this.conclusion));

        fields.add_field_method_get("formal", |_, this| Ok(Self::new(this.out().formal())));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "instantiateTemplates",
            |lua, this, templates: Vec<LuaPresFrag>| {
                let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
                let fact = this.out();
                let templates = &|idx: usize| templates[idx].out();
                let assumption = fact
                    .assumption()
                    .map(|a| instantiate_templates(a, templates, ctx));
                let conclusion = instantiate_templates(fact.conclusion(), templates, ctx);
                let fact = PresFact::new(assumption, conclusion);
                Ok(LuaPresFact::new(fact))
            },
        );

        methods.add_meta_method(MetaMethod::ToString, |_, this, _args: ()| {
            Ok(this.out().print())
        });

        methods.add_meta_method(MetaMethod::Eq, |_, this, other: Self| {
            Ok(this.out() == other.out())
        });
    }
}

pub struct LuaPresFactMeta;

impl UserData for LuaPresFactMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "new",
            |_, _, (assumption, conclusion): (Option<LuaPresFrag>, LuaPresFrag)| {
                let assumption = assumption.map(|a| a.out());
                let conclusion = conclusion.out();
                let fact = PresFact::new(assumption, conclusion);
                Ok(LuaPresFact::new(fact))
            },
        );

        methods.add_method("newC", |_, _, conclusion: LuaPresFrag| {
            let fact = PresFact::new(None, conclusion.out());
            Ok(LuaPresFact::new(fact))
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct LuaFrag {
    ptr: *const Fragment<'static>,
}

impl LuaFrag {
    pub fn new<'ctx>(frag: FragmentId<'ctx>) -> Self {
        let ptr = std::ptr::from_ref(frag.0);
        // SAFETY: We don't use this ptr here so this isn't really unsafe. See
        // where we dereference for the actual safety details.
        let ptr: *const Fragment<'static> = unsafe { std::mem::transmute(ptr) };

        Self { ptr }
    }

    pub fn out<'ctx>(&self) -> FragmentId<'ctx> {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let frag = unsafe { &*self.ptr };
        FragmentId(frag)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LuaPres {
    ptr: *const Pres<'static>,
}

impl LuaPres {
    pub fn new<'ctx>(pres: PresId<'ctx>) -> Self {
        let ptr = std::ptr::from_ref(pres.0);
        // SAFETY: We don't use this ptr here so this isn't really unsafe. See
        // where we dereference for the actual safety details.
        let ptr: *const Pres<'static> = unsafe { std::mem::transmute(ptr) };

        Self { ptr }
    }

    pub fn out<'ctx>(&self) -> PresId<'ctx> {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let pres = unsafe { &*self.ptr };
        PresId(pres)
    }
}
