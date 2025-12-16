use crate::semant::{
    fragment::{Fragment, FragmentId},
    presentation::{Pres, PresFrag, PresId},
    theorems::PresFact,
};
use mlua::{FromLua, MetaMethod, UserData};

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
        fields.add_field_method_get("formal", |_, this| {
            Ok(LuaPresFrag::new(this.out().formal()))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, _args: ()| {
            Ok(this.out().print())
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
        methods.add_meta_method(MetaMethod::ToString, |_, this, _args: ()| {
            Ok(this.out().print())
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
