use crate::semant::{
    fragment::{Fragment, FragmentId},
    presentation::{PresFrag, PresTree, PresTreeId},
    theorems::PresFact,
};
use mlua::{FromLua, MetaMethod, UserData};

#[derive(Debug, Clone, Copy, FromLua)]
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

impl UserData for LuaFrag {}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaPresTree {
    ptr: *const PresTree<'static>,
}

impl LuaPresTree {
    pub fn new<'ctx>(pres: PresTreeId<'ctx>) -> Self {
        let ptr = std::ptr::from_ref(pres.0);
        // SAFETY: We don't use this ptr here so this isn't really unsafe. See
        // where we dereference for the actual safety details.
        let ptr: *const PresTree<'static> = unsafe { std::mem::transmute(ptr) };

        Self { ptr }
    }

    pub fn out<'ctx>(&self) -> PresTreeId<'ctx> {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let pres = unsafe { &*self.ptr };
        PresTreeId(pres)
    }
}

impl UserData for LuaPresTree {}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaPresFrag {
    frag: LuaFrag,
    pres: LuaPresTree,
}

impl LuaPresFrag {
    pub fn new<'ctx>(pres_frag: PresFrag<'ctx>) -> Self {
        Self {
            frag: LuaFrag::new(pres_frag.frag()),
            pres: LuaPresTree::new(pres_frag.pres()),
        }
    }

    pub fn out<'ctx>(&self) -> PresFrag<'ctx> {
        PresFrag(self.frag.out(), self.pres.out())
    }
}

impl UserData for LuaPresFrag {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(this.out().pres().pres().print())
        });
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaPresFact {
    assumption: Option<LuaPresFrag>,
    conclusion: LuaPresFrag,
}

impl LuaPresFact {
    pub fn new<'ctx>(fact: &PresFact<'ctx>) -> Self {
        Self {
            assumption: fact.assumption().map(LuaPresFrag::new),
            conclusion: LuaPresFrag::new(fact.conclusion()),
        }
    }

    pub fn out<'ctx>(&self) -> PresFact<'ctx> {
        PresFact::new(self.assumption.map(|a| a.out()), self.conclusion.out())
    }
}

impl UserData for LuaPresFact {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, _: ()| {
            Ok(this.out().print())
        });
    }
}
