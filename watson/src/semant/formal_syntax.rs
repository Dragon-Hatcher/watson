use std::ops::Index;

use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use ustr::Ustr;

use crate::{
    parse::parse_state::{Associativity, Precedence},
    strings,
};

#[derive(Debug, Clone)]
pub struct FormalSyntax {
    cats: SlotMap<FormalSyntaxCatId, FormalSyntaxCat>,
    cats_by_name: FxHashMap<Ustr, FormalSyntaxCatId>,
    sentence_cat: FormalSyntaxCatId,

    rules: SlotMap<FormalSyntaxRuleId, FormalSyntaxRule>,
    rules_by_name: FxHashMap<Ustr, FormalSyntaxRuleId>,
}

impl FormalSyntax {
    pub fn new() -> Self {
        let mut cats = SlotMap::default();
        let sentence_cat = cats.insert(FormalSyntaxCat::new(*strings::SENTENCE));
        let mut cats_by_name = FxHashMap::default();
        cats_by_name.insert(*strings::SENTENCE, sentence_cat);

        Self {
            cats,
            cats_by_name,
            sentence_cat,
            rules: SlotMap::default(),
            rules_by_name: FxHashMap::default(),
        }
    }

    pub fn add_cat(&mut self, cat: FormalSyntaxCat) -> FormalSyntaxCatId {
        let name = cat.name;
        assert!(!self.cats_by_name.contains_key(&name));
        let id = self.cats.insert(cat);
        self.cats_by_name.insert(name, id);
        id
    }

    pub fn cat_by_name(&self, name: Ustr) -> Option<FormalSyntaxCatId> {
        self.cats_by_name.get(&name).copied()
    }

    pub fn sentence_cat(&self) -> FormalSyntaxCatId {
        self.sentence_cat
    }

    pub fn add_rule(&mut self, rule: FormalSyntaxRule) -> FormalSyntaxRuleId {
        let name = rule.name;
        assert!(!self.rules_by_name.contains_key(&name));
        let id = self.rules.insert(rule);
        self.rules_by_name.insert(name, id);
        id
    }

    pub fn rule_by_name(&self, name: Ustr) -> Option<FormalSyntaxRuleId> {
        self.rules_by_name.get(&name).copied()
    }
}

impl Index<FormalSyntaxCatId> for FormalSyntax {
    type Output = FormalSyntaxCat;

    fn index(&self, index: FormalSyntaxCatId) -> &Self::Output {
        &self.cats[index]
    }
}

impl Index<FormalSyntaxRuleId> for FormalSyntax {
    type Output = FormalSyntaxRule;

    fn index(&self, index: FormalSyntaxRuleId) -> &Self::Output {
        &self.rules[index]
    }
}

new_key_type! { pub struct FormalSyntaxCatId; }
new_key_type! { pub struct FormalSyntaxRuleId; }

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
pub struct FormalSyntaxRule {
    name: Ustr,
    cat: FormalSyntaxCatId,
    pat: FormalSyntaxPat,
}

impl FormalSyntaxRule {
    pub fn new(name: Ustr, cat: FormalSyntaxCatId, pat: FormalSyntaxPat) -> Self {
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
pub struct FormalSyntaxPat {
    parts: Vec<FormalSyntaxPatPart>,
    precedence: Precedence,
    associativity: Associativity,
}

impl FormalSyntaxPat {
    pub fn new(parts: Vec<FormalSyntaxPatPart>) -> Self {
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
pub enum FormalSyntaxPatPart {
    Cat(FormalSyntaxCatId),
    Binding(FormalSyntaxCatId),
    Var(FormalSyntaxCatId),
    Lit(Ustr),
}
