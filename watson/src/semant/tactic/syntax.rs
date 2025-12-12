use crate::{
    generate_arena_handle,
    parse::parse_state::{Associativity, CategoryId, Precedence},
};
use ustr::Ustr;

generate_arena_handle!(TacticCatId<'ctx> => TacticCat);
generate_arena_handle!(TacticRuleId<'ctx> => TacticRule<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TacticCat {
    name: Ustr,
}

impl TacticCat {
    pub fn new(name: Ustr) -> Self {
        Self { name }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TacticRule<'ctx> {
    name: Ustr,
    cat: TacticCatId<'ctx>,
    pat: TacticPat<'ctx>,
}

impl<'ctx> TacticRule<'ctx> {
    pub fn new(name: Ustr, cat: TacticCatId<'ctx>, pat: TacticPat<'ctx>) -> Self {
        Self { name, cat, pat }
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TacticPat<'ctx> {
    parts: Vec<TacticPatPart<'ctx>>,
    precedence: Precedence,
    associativity: Associativity,
}

impl<'ctx> TacticPat<'ctx> {
    pub fn new(parts: Vec<TacticPatPart<'ctx>>, precedence: Precedence, associativity: Associativity) -> Self {
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
