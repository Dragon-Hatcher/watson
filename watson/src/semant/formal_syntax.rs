use std::ops::Index;

use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use ustr::Ustr;

use crate::strings;

#[derive(Debug, Clone)]
pub struct FormalSyntax {
    cats: SlotMap<FormalSyntaxCatId, FormalSyntaxCat>,
    cats_by_name: FxHashMap<Ustr, FormalSyntaxCatId>,
    sentence_cat: FormalSyntaxCatId,

    rules: SlotMap<FormalSyntaxRuleId, FormalSyntaxRule>,
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
        self.cats_by_name.get(&name).cloned()
    }

    pub fn sentence_cat(&self) -> FormalSyntaxCatId {
        self.sentence_cat
    }

    pub fn add_rule(&mut self, cat: FormalSyntaxCatId) -> FormalSyntaxRuleId {
        self.rules.insert(FormalSyntaxRule { cat })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormalSyntaxRule {
    cat: FormalSyntaxCatId,
}
