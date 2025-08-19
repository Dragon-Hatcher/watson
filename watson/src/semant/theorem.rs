use crate::semant::formal_syntax::FormalSyntaxCatId;
use std::{collections::HashMap, hash::Hash};
use ustr::Ustr;

pub struct Theorems {
    theorems: HashMap<TheoremId, Theorem>,
}

impl Theorems {
    pub fn new() -> Self {
        Self {
            theorems: HashMap::new(),
        }
    }

    pub fn has(&self, id: TheoremId) -> bool {
        self.theorems.contains_key(&id)
    }

    pub fn add(&mut self, theorem: Theorem) {
        self.theorems.insert(theorem.id.clone(), theorem);
    }

    pub fn get(&self, id: &TheoremId) -> Option<&Theorem> {
        self.theorems.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Theorem> {
        self.theorems.values()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TheoremId(Ustr);

impl TheoremId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone)]
pub struct Theorem {
    id: TheoremId,
    templates: Vec<Template>,
    hypotheses: Vec<Sentence>,
    conclusion: Sentence,
    proof: Proof,
}

impl Theorem {
    pub fn new(
        id: TheoremId,
        templates: Vec<Template>,
        hypotheses: Vec<Sentence>,
        conclusion: Sentence,
        proof: Proof,
    ) -> Self {
        Self {
            id,
            templates,
            hypotheses,
            conclusion,
            proof,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Proof {
    Axiom,
    Theorem,
}

#[derive(Debug, Clone)]
pub struct Sentence;

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