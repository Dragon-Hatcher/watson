use std::collections::HashMap;
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct FormalSyntax {
    categories: HashMap<FormalSyntaxCatId, ()>,
    rules: HashMap<FormalSyntaxRuleId, (FormalSyntaxCatId, FormalSyntaxPattern)>,
}

impl FormalSyntax {
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
            rules: HashMap::new(),
        }
    }

    pub fn has_cat(&self, id: FormalSyntaxCatId) -> bool {
        self.categories.contains_key(&id)
    }

    pub fn add_cat(&mut self, id: FormalSyntaxCatId) {
        self.categories.insert(id, ());
    }

    pub fn has_rule(&self, id: FormalSyntaxRuleId) -> bool {
        self.rules.contains_key(&id)
    }

    pub fn add_rule(
        &mut self,
        id: FormalSyntaxRuleId,
        cat: FormalSyntaxCatId,
        pattern: FormalSyntaxPattern,
    ) {
        self.rules.insert(id, (cat, pattern));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormalSyntaxCatId(Ustr);

impl FormalSyntaxCatId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormalSyntaxRuleId(Ustr);

impl FormalSyntaxRuleId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalSyntaxPattern {
    parts: Vec<FormalSyntaxPatternPart>,
}

impl FormalSyntaxPattern {
    pub fn new(parts: Vec<FormalSyntaxPatternPart>) -> Self {
        Self { parts }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormalSyntaxPatternPart {
    Cat(FormalSyntaxCatId),
    Lit(Ustr),
    Binding,
    Variable,
}
