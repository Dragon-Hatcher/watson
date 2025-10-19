use ustr::Ustr;

use crate::{
    generate_arena_handle,
    parse::parse_state::{Associativity, Precedence},
};

generate_arena_handle! { FormalSyntaxCatId => FormalSyntaxCat }
generate_arena_handle! { FormalSyntaxRuleId<'ctx> => FormalSyntaxRule<'ctx> }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormalSyntaxCat {
    name: Ustr,
}

impl FormalSyntaxCat {
    pub fn new(name: Ustr) -> Self {
        Self { name }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormalSyntaxRule<'ctx> {
    name: Ustr,
    cat: FormalSyntaxCatId<'ctx>,
    pat: FormalSyntaxPat<'ctx>,
}

impl<'ctx> FormalSyntaxRule<'ctx> {
    pub fn new(name: Ustr, cat: FormalSyntaxCatId<'ctx>, pat: FormalSyntaxPat<'ctx>) -> Self {
        Self { name, cat, pat }
    }

    pub fn _name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn pattern(&self) -> &FormalSyntaxPat<'ctx> {
        &self.pat
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormalSyntaxPat<'ctx> {
    parts: Vec<FormalSyntaxPatPart<'ctx>>,
    precedence: Precedence,
    associativity: Associativity,
}

impl<'ctx> FormalSyntaxPat<'ctx> {
    pub fn new(parts: Vec<FormalSyntaxPatPart<'ctx>>) -> Self {
        Self {
            parts,
            precedence: Precedence(0),
            associativity: Associativity::NonAssoc,
        }
    }

    pub fn parts(&self) -> &[FormalSyntaxPatPart<'ctx>] {
        &self.parts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormalSyntaxPatPart<'ctx> {
    Cat(FormalSyntaxCatId<'ctx>),
    Binding(FormalSyntaxCatId<'ctx>),
    Var(FormalSyntaxCatId<'ctx>),
    Lit(Ustr),
}
