use crate::{
    diagnostics::DiagnosticSpan, parse::Span,
    semant::check_proofs::lua_api::diag_to_lua::LuaDiagnosticSpan,
};
use mlua::{FromLua, UserData};
use ustr::Ustr;

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

impl UserData for LuaSpan {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("label", |_, this, msg: String| {
            let msg = Ustr::from(&msg);
            let d_span = DiagnosticSpan::new_error(msg.as_str(), this.out());
            Ok(LuaDiagnosticSpan::new(d_span))
        });

        methods.add_method("labelInfo", |_, this, msg: String| {
            let msg = Ustr::from(&msg);
            let d_span = DiagnosticSpan::new_info(msg.as_str(), this.out());
            Ok(LuaDiagnosticSpan::new(d_span))
        });
    }
}
