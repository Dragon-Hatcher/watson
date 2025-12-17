use crate::semant::notation::NotationBindingId;
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

impl UserData for LuaNotationBinding {}
