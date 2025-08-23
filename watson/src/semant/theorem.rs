use crate::semant::{formal_syntax::FormalSyntaxCatId, fragments::FragId};
use std::collections::HashMap;
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TheoremId(Ustr);

impl TheoremId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }

    pub fn name(&self) -> Ustr {
        self.0
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

    pub fn get(&self, id: TheoremId) -> &TheoremStatement {
        &self.theorems[&id]
    }

    pub fn has(&self, id: TheoremId) -> bool {
        self.theorems.contains_key(&id)
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

    pub fn _id(&self) -> TheoremId {
        self.id
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    pub fn hypotheses(&self) -> &[Fact] {
        &self.hypotheses
    }

    pub fn conclusion(&self) -> FragId {
        self.conclusion
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn assumption(&self) -> Option<FragId> {
        self.assumption
    }

    pub fn sentence(&self) -> FragId {
        self.sentence
    }
}
