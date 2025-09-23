use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::semant::{formal_syntax::FormalSyntaxCatId, fragment::FragmentId};

pub struct TheoremStatements {
    theorems: FxHashMap<Ustr, TheoremStatement>,
}

impl TheoremStatements {
    pub fn new() -> Self {
        Self {
            theorems: FxHashMap::default(),
        }
    }

    pub fn add(&mut self, name: Ustr, statement: TheoremStatement) {
        self.theorems.insert(name, statement);
    }

    pub fn get(&self, name: Ustr) -> Option<&TheoremStatement> {
        self.theorems.get(&name)
    }
}

pub struct TheoremStatement {
    templates: FxHashMap<Ustr, Template>,
    hypotheses: Vec<Fact>,
    conclusion: FragmentId,
}

impl TheoremStatement {
    pub fn new(
        templates: FxHashMap<Ustr, Template>,
        hypotheses: Vec<Fact>,
        conclusion: FragmentId,
    ) -> Self {
        Self {
            templates,
            hypotheses,
            conclusion,
        }
    }

    pub fn templates(&self) -> &FxHashMap<Ustr, Template> {
        &self.templates
    }

    pub fn hypotheses(&self) -> &[Fact] {
        &self.hypotheses
    }

    pub fn conclusion(&self) -> FragmentId {
        self.conclusion
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fact {
    assumption: Option<FragmentId>,
    conclusion: FragmentId,
}

impl Fact {
    pub fn new(assumption: Option<FragmentId>, conclusion: FragmentId) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }

    pub fn assumption(&self) -> Option<FragmentId> {
        self.assumption
    }

    pub fn conclusion(&self) -> FragmentId {
        self.conclusion
    }
}
