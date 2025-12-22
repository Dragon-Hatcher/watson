use crate::{
    context::arena::ScopeId,
    generate_arena_handle,
    parse::parse_state::{Associativity, CategoryId, Precedence},
    util::name_to_lua,
};
use ustr::Ustr;

generate_arena_handle!(TacticCatId<'ctx> => TacticCat);
generate_arena_handle!(TacticRuleId<'ctx> => TacticRule<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TacticCat {
    name: Ustr,
    lua_name: Ustr,
}

impl TacticCat {
    pub fn new(name: Ustr) -> Self {
        Self {
            name,
            lua_name: Ustr::from(&name_to_lua(&name)),
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn lua_name(&self) -> Ustr {
        self.lua_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TacticRule<'ctx> {
    name: Ustr,
    cat: TacticCatId<'ctx>,
    pat: TacticPat<'ctx>,
    scope: ScopeId,
}

impl<'ctx> TacticRule<'ctx> {
    pub fn new(name: Ustr, cat: TacticCatId<'ctx>, pat: TacticPat<'ctx>, scope: ScopeId) -> Self {
        Self {
            name,
            cat,
            pat,
            scope,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> TacticCatId<'ctx> {
        self.cat
    }

    pub fn pattern(&self) -> &TacticPat<'ctx> {
        &self.pat
    }

    pub fn scope(&self) -> ScopeId {
        self.scope
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TacticPat<'ctx> {
    parts: Vec<TacticPatPart<'ctx>>,
    precedence: Precedence,
    associativity: Associativity,
}

impl<'ctx> TacticPat<'ctx> {
    pub fn new(
        parts: Vec<TacticPatPart<'ctx>>,
        precedence: Precedence,
        associativity: Associativity,
    ) -> Self {
        Self {
            parts,
            precedence,
            associativity,
        }
    }

    pub fn parts(&self) -> &[TacticPatPart<'ctx>] {
        &self.parts
    }

    pub fn precedence(&self) -> Precedence {
        self.precedence
    }

    pub fn associativity(&self) -> Associativity {
        self.associativity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TacticPatPart<'ctx> {
    label: Option<Ustr>,
    part: TacticPatPartCore<'ctx>,
}

impl<'ctx> TacticPatPart<'ctx> {
    pub fn new(label: Option<Ustr>, part: TacticPatPartCore<'ctx>) -> Self {
        Self { label, part }
    }

    pub fn label(&self) -> Option<Ustr> {
        self.label
    }

    pub fn part(&self) -> &TacticPatPartCore<'ctx> {
        &self.part
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TacticPatPartCore<'ctx> {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Cat(TacticCatId<'ctx>),
    Frag(CategoryId<'ctx>),
    AnyFrag,
    Fact,
}
