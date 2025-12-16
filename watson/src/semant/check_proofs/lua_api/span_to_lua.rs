use crate::parse::Span;
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaSpan(Span);

impl LuaSpan {
    pub fn new(span: Span) -> Self {
        Self(span)
    }

    pub fn out(&self) -> Span {
        self.0
    }
}

impl UserData for LuaSpan {}
