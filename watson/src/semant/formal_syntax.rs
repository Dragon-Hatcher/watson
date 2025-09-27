use std::ops::Index;

use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use ustr::Ustr;

use crate::{
    context::arena::NamedArena,
    declare_intern_handle,
    parse::parse_state::{Associativity, Precedence},
    strings,
};

pub struct FormalSyntax<'ctx> {
    cats: NamedArena<FormalSyntaxCat, FormalSyntaxCatId<'ctx>>,
    rules: NamedArena<FormalSyntaxRule<'ctx>, FormalSyntaxRuleId<'ctx>>,
    sentence_cat: FormalSyntaxCatId<'ctx>,
}

impl<'ctx> FormalSyntax<'ctx> {
    pub fn new() -> Self {
        let cats = NamedArena::new();
        let sentence_cat = FormalSyntaxCat::new(*strings::SENTENCE);
        let sentence_cat = cats.alloc(sentence_cat.name, sentence_cat);

        Self {
            cats,
            rules: NamedArena::new(),
            sentence_cat,
        }
    }

    pub fn add_cat(&'ctx self, cat: FormalSyntaxCat) -> FormalSyntaxCatId {
        assert!(self.cats.get(cat.name).is_none());
        self.cats.alloc(cat.name, cat)
    }

    pub fn cat_by_name(&self, name: Ustr) -> Option<FormalSyntaxCatId> {
        self.cats.get(name)
    }

    pub fn sentence_cat(&self) -> FormalSyntaxCatId {
        self.sentence_cat
    }

    pub fn add_rule(&'ctx self, rule: FormalSyntaxRule<'ctx>) -> FormalSyntaxRuleId<'ctx> {
        assert!(self.rules.get(rule.name).is_none());
        self.rules.alloc(rule.name, rule)
    }

    pub fn rule_by_name(&self, name: Ustr) -> Option<FormalSyntaxRuleId> {
        self.rules.get(name)
    }
}

declare_intern_handle! { FormalSyntaxCatId => FormalSyntaxCat }
declare_intern_handle! { FormalSyntaxRuleId<'ctx> => FormalSyntaxRule<'ctx> }

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

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> FormalSyntaxCatId {
        self.cat
    }

    pub fn pattern(&self) -> &FormalSyntaxPat {
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

    pub fn parts(&self) -> &[FormalSyntaxPatPart] {
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
