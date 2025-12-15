use crate::context::Ctx;

#[derive(Clone, Copy)]
pub struct LuaCtx {
    ctx: &'static Ctx<'static>,
}

impl LuaCtx {
    pub fn new<'ctx>(ctx: &Ctx<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let ctx: &'static Ctx<'static> = unsafe { std::mem::transmute(ctx) };
        Self { ctx }
    }

    pub fn out<'ctx, 'a>(self) -> &'a Ctx<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.ctx) }
    }
}
