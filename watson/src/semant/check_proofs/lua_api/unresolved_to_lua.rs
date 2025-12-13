use crate::{
    parse::parse_tree::{ParseTree, ParseTreeId},
    semant::parse_fragment::{UnresolvedAnyFrag, UnresolvedFact, UnresolvedFrag},
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

impl UserData for LuaUnresolvedFrag {}

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

impl UserData for LuaUnresolvedAnyFrag {}

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

impl UserData for LuaUnresolvedFact {}
