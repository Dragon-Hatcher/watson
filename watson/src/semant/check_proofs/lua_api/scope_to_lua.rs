use crate::semant::scope::Scope;
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

impl UserData for LuaScope {}
