use std::collections::HashMap;

use crate::semant::{
    formal_syntax::FormalSyntaxCatId,
    fragments::{Frag, FragId},
};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TheoremId(Ustr);

impl TheoremId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone)]
pub struct Template {
    name: Ustr,
    cat: FormalSyntaxCatId,
    params: Vec<FormalSyntaxCatId>,
}

impl Template {
    pub fn new(name: Ustr, cat: FormalSyntaxCatId, params: Vec<FormalSyntaxCatId>) -> Self {
        Self { name, cat, params }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> FormalSyntaxCatId {
        self.cat
    }

    pub fn params(&self) -> &[FormalSyntaxCatId] {
        &self.params
    }
}

#[derive(Debug, Clone)]
pub struct TheoremStatements {
    theorems: HashMap<TheoremId, TheoremStatement>,
}

impl TheoremStatements {
    pub fn new() -> Self {
        Self {
            theorems: HashMap::new(),
        }
    }

    pub fn add(&mut self, theorem: TheoremStatement) {
        self.theorems.insert(theorem.id, theorem);
    }
}

#[derive(Debug, Clone)]
pub struct TheoremStatement {
    id: TheoremId,
    templates: Vec<Template>,
    hypotheses: Vec<Fact>,
    conclusion: FragId,
}

impl TheoremStatement {
    pub fn new(
        id: TheoremId,
        templates: Vec<Template>,
        hypotheses: Vec<Fact>,
        conclusion: FragId,
    ) -> Self {
        Self {
            id,
            templates,
            hypotheses,
            conclusion,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fact {
    assumption: Option<FragId>,
    sentence: FragId,
}

impl Fact {
    pub fn new(assumption: Option<FragId>, sentence: FragId) -> Self {
        Self {
            assumption,
            sentence,
        }
    }
}
