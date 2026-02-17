use mlua::{FromLua, MetaMethod, UserData};

use crate::semant::commands::CommandId;

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaCommandId {
    id: CommandId<'static>,
}

impl LuaCommandId {
    pub fn new<'ctx>(id: CommandId<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let id: CommandId<'static> = unsafe { std::mem::transmute(id) };
        Self { id }
    }

    pub fn out<'ctx>(&self) -> CommandId<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.id) }
    }
}

impl UserData for LuaCommandId {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Eq, |_, this, other: LuaCommandId| {
            Ok(this.out() == other.out())
        });
    }
}
