use crate::{
    diagnostics::Diagnostic,
    semant::check_proofs::{
        LuaTheoremInfo,
        lua_api::{span_to_lua::LuaSpan, tactic_info_to_lua::LuaTacticInfo},
    },
};
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, FromLua)]
pub struct LuaDiagnostic {
    diag: Diagnostic<'static>,
}

impl LuaDiagnostic {
    pub fn new<'ctx>(diag: Diagnostic<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let diag: Diagnostic<'static> = unsafe { std::mem::transmute(diag) };
        Self { diag }
    }

    pub fn out<'ctx>(self) -> Diagnostic<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(self.diag) }
    }
}

impl UserData for LuaDiagnostic {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("withError", |_, this, (msg, span): (String, LuaSpan)| {
            let new_diag = this.clone().out().with_error(&msg, span.out());
            Ok(LuaDiagnostic::new(new_diag))
        });

        methods.add_method("withInfo", |_, this, (msg, span): (String, LuaSpan)| {
            let new_diag = this.clone().out().with_info(&msg, span.out());
            Ok(LuaDiagnostic::new(new_diag))
        });

        methods.add_method("withTacticInfo", |lua, this, tactic_info: LuaTacticInfo| {
            let info = lua.app_data_ref::<LuaTheoremInfo>().unwrap();
            let thm = info.borrow().thm.out();
            let tactic_info = tactic_info.out_ref().clone();
            let new_diag = this.clone().out().in_proof(thm, tactic_info);
            Ok(LuaDiagnostic::new(new_diag))
        });

        methods.add_method("report", |lua, this, _: ()| {
            let info = lua.app_data_ref::<LuaTheoremInfo>().unwrap();
            info.borrow_mut().diags.push(this.clone());
            Ok(())
        });
    }
}

pub struct LuaDiagnosticMeta;

impl UserData for LuaDiagnosticMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, title: String| {
            let diag = Diagnostic::new(&title);
            Ok(LuaDiagnostic::new(diag))
        });
    }
}
