use crate::strings;
use std::collections::HashMap;
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct FormalSyntax {
    categories: HashMap<FormalSyntaxCatId, ()>,
    rules: HashMap<FormalSyntaxRuleId, FormalSyntaxRule>,
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

    pub fn cats(&self) -> impl Iterator<Item = &FormalSyntaxCatId> {
        self.categories.keys()
    }

    pub fn has_rule(&self, id: FormalSyntaxRuleId) -> bool {
        self.rules.contains_key(&id)
    }

    pub fn add_rule(&mut self, rule: FormalSyntaxRule) {
        self.rules.insert(rule.id(), rule);
    }

    pub fn get_rule(&self, rule: FormalSyntaxRuleId) -> &FormalSyntaxRule {
        &self.rules[&rule]
    }

    pub fn rules(&self) -> impl Iterator<Item = &FormalSyntaxRule> {
        self.rules.values()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormalSyntaxCatId(Ustr);

impl FormalSyntaxCatId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }

    pub fn sentence() -> Self {
        Self::new(*strings::SENTENCE)
    }

    pub fn name(&self) -> Ustr {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormalSyntaxRuleId(Ustr);

impl FormalSyntaxRuleId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalSyntaxRule {
    cat: FormalSyntaxCatId,
    rule: FormalSyntaxRuleId,
    pat: FormalSyntaxPattern,
}

impl FormalSyntaxRule {
    pub fn new(cat: FormalSyntaxCatId, rule: FormalSyntaxRuleId, pat: FormalSyntaxPattern) -> Self {
        Self { cat, rule, pat }
    }

    pub fn cat(&self) -> FormalSyntaxCatId {
        self.cat
    }

    pub fn id(&self) -> FormalSyntaxRuleId {
        self.rule
    }

    pub fn pat(&self) -> &FormalSyntaxPattern {
        &self.pat
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

    pub fn parts(&self) -> &[FormalSyntaxPatternPart] {
        &self.parts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormalSyntaxPatternPart {
    Cat(FormalSyntaxCatId),
    Lit(Ustr),
    Binding(FormalSyntaxCatId),
    Variable(FormalSyntaxCatId),
}
