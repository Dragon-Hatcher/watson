use crate::{
    diagnostics::Diagnostic,
    parse::parse_tree::{ParseTree, ParseTreeId},
    semant::{
        check_proofs::lua_api::{
            ctx_to_lua::LuaCtx,
            diag_to_lua::LuaDiagnostic,
            frag_to_lua::{LuaPresFact, LuaPresFrag},
            scope_to_lua::LuaScope,
            span_to_lua::LuaSpan,
        },
        parse_fragment::{UnresolvedAnyFrag, UnresolvedFact, UnresolvedFrag, parse_fragment},
        theorems::PresFact,
    },
};
use mlua::{FromLua, UserData};

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaUnresolvedFrag {
    ptr: *const ParseTree<'static>,
}

impl LuaUnresolvedFrag {
    pub fn new<'ctx>(frag: UnresolvedFrag<'ctx>) -> Self {
        let ptr = std::ptr::from_ref(frag.0.0);
        // SAFETY: We don't use this ptr here so this isn't really unsafe. See
        // where we dereference for the actual safety details.
        let ptr: *const ParseTree<'static> = unsafe { std::mem::transmute(ptr) };

        Self { ptr }
    }

    pub fn out<'ctx>(&self) -> UnresolvedFrag<'ctx> {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let parse_tree = unsafe { &*self.ptr };
        let parse_tree_id = ParseTreeId(parse_tree);
        UnresolvedFrag(parse_tree_id)
    }
}

impl UserData for LuaUnresolvedFrag {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("span", |_, this| {
            let span = this.out().0.span();
            Ok(LuaSpan::new(span))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("resolve", |lua, this, scope: LuaScope| {
            let un_frag = this.out();
            let scope = scope.out_ref();
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();
            let frag = parse_fragment(un_frag, scope, ctx).expect("TODO");

            let res = match frag {
                Ok(frag) => (Some(LuaPresFrag::new(frag)), None),
                Err(err) => {
                    let diag = Diagnostic::err_frag_parse_failure(un_frag.0.span(), err);
                    (None, Some(LuaDiagnostic::new(diag)))
                }
            };
            Ok(res)
        });
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaUnresolvedAnyFrag {
    ptr: *const ParseTree<'static>,
}

impl LuaUnresolvedAnyFrag {
    pub fn new<'ctx>(frag: UnresolvedAnyFrag<'ctx>) -> Self {
        let ptr = std::ptr::from_ref(frag.0.0);
        let ptr: *const ParseTree<'static> = unsafe { std::mem::transmute(ptr) };

        Self { ptr }
    }

    pub fn out<'ctx>(&self) -> UnresolvedAnyFrag<'ctx> {
        let parse_tree = unsafe { &*self.ptr };
        let parse_tree_id = ParseTreeId(parse_tree);
        UnresolvedAnyFrag(parse_tree_id)
    }
}

impl UserData for LuaUnresolvedAnyFrag {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("span", |_, this| {
            let span = this.out().0.span();
            Ok(LuaSpan::new(span))
        });
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaUnresolvedFact {
    assumption: Option<LuaUnresolvedFrag>,
    conclusion: LuaUnresolvedFrag,
}

impl LuaUnresolvedFact {
    pub fn new<'ctx>(fact: &UnresolvedFact<'ctx>) -> Self {
        Self {
            assumption: fact.assumption.map(LuaUnresolvedFrag::new),
            conclusion: LuaUnresolvedFrag::new(fact.conclusion),
        }
    }

    pub fn out<'ctx>(&self) -> UnresolvedFact<'ctx> {
        UnresolvedFact {
            assumption: self.assumption.map(|a| a.out()),
            conclusion: self.conclusion.out(),
        }
    }
}

impl UserData for LuaUnresolvedFact {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("assumption", |_, this| Ok(this.assumption));

        fields.add_field_method_get("conclusion", |_, this| Ok(this.conclusion));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("resolve", |lua, this, scope: LuaScope| {
            let scope = scope.out_ref();
            let ctx = lua.app_data_ref::<LuaCtx>().unwrap().out();

            let un_conclusion = this.out().conclusion;
            let conclusion = parse_fragment(un_conclusion, scope, ctx).expect("TODO");
            let (conclusion, conclusion_diag) = match conclusion {
                Ok(frag) => (Some(frag), None),
                Err(err) => {
                    let diag = Diagnostic::err_frag_parse_failure(un_conclusion.0.span(), err);
                    (None, Some(LuaDiagnostic::new(diag)))
                }
            };

            let un_assumption = this.out().assumption;
            let assumption = un_assumption.map(|a| parse_fragment(a, scope, ctx).expect("TODO"));
            let (assumption, assumption_diag) = match assumption {
                None => (Some(None), None),
                Some(Ok(frag)) => (Some(Some(frag)), None),
                Some(Err(err)) => {
                    let diag = Diagnostic::err_frag_parse_failure(un_conclusion.0.span(), err);
                    (None, Some(LuaDiagnostic::new(diag)))
                }
            };

            let res = match (assumption, conclusion) {
                (Some(assumption), Some(conclusion)) => {
                    let fact = PresFact::new(assumption, conclusion);
                    Some(LuaPresFact::new(fact))
                }
                _ => None,
            };
            Ok((res, assumption_diag, conclusion_diag))
        });
    }
}
