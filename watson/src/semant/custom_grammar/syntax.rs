use crate::{
    context::arena::ScopeId,
    generate_arena_handle,
    parse::parse_state::{Associativity, CategoryId, Precedence},
    util::name_to_lua,
};
use ustr::Ustr;

generate_arena_handle!(CustomGrammarCatId<'ctx> => CustomGrammarCat);
generate_arena_handle!(CustomGrammarRuleId<'ctx> => CustomGrammarRule<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CustomGrammarCat {
    name: Ustr,
    lua_name: Ustr,
}

impl CustomGrammarCat {
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
pub struct CustomGrammarRule<'ctx> {
    name: Ustr,
    cat: CustomGrammarCatId<'ctx>,
    pat: CustomGrammarPat<'ctx>,
    scope: ScopeId,
}

impl<'ctx> CustomGrammarRule<'ctx> {
    pub fn new(
        name: Ustr,
        cat: CustomGrammarCatId<'ctx>,
        pat: CustomGrammarPat<'ctx>,
        scope: ScopeId,
    ) -> Self {
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

    pub fn cat(&self) -> CustomGrammarCatId<'ctx> {
        self.cat
    }

    pub fn pattern(&self) -> &CustomGrammarPat<'ctx> {
        &self.pat
    }

    pub fn scope(&self) -> ScopeId {
        self.scope
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomGrammarPat<'ctx> {
    parts: Vec<CustomGrammarPatPart<'ctx>>,
    precedence: Precedence,
    associativity: Associativity,
}

impl<'ctx> CustomGrammarPat<'ctx> {
    pub fn new(
        parts: Vec<CustomGrammarPatPart<'ctx>>,
        precedence: Precedence,
        associativity: Associativity,
    ) -> Self {
        Self {
            parts,
            precedence,
            associativity,
        }
    }

    pub fn parts(&self) -> &[CustomGrammarPatPart<'ctx>] {
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
pub struct CustomGrammarPatPart<'ctx> {
    label: Option<Ustr>,
    part: CustomGrammarPatPartCore<'ctx>,
}

impl<'ctx> CustomGrammarPatPart<'ctx> {
    pub fn new(label: Option<Ustr>, part: CustomGrammarPatPartCore<'ctx>) -> Self {
        Self { label, part }
    }

    pub fn label(&self) -> Option<Ustr> {
        self.label
    }

    pub fn part(&self) -> &CustomGrammarPatPartCore<'ctx> {
        &self.part
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomGrammarPatPartCore<'ctx> {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Cat(CustomGrammarCatId<'ctx>),
    Frag(CategoryId<'ctx>),
    AnyFrag,
    Fact,
}
