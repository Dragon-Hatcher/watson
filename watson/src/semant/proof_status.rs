use std::ops::Index;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::semant::{proof_kernel::ProofCertificate, theorems::TheoremId};

#[derive(Debug)]
pub struct ProofStatuses<'ctx> {
    statuses: FxHashMap<TheoremId<'ctx>, ProofStatus<'ctx>>,
    theorem_cnt: usize,
    axiom_cnt: usize,
    correct_cnt: usize,
    todo_cnt: usize,
    /// Maps each todo reason to the number of theorems that used todo with that reason.
    todo_by_reason: FxHashMap<Option<String>, usize>,
}

impl<'ctx> ProofStatuses<'ctx> {
    pub fn new() -> Self {
        Self {
            statuses: FxHashMap::default(),
            theorem_cnt: 0,
            axiom_cnt: 0,
            correct_cnt: 0,
            todo_cnt: 0,
            todo_by_reason: FxHashMap::default(),
        }
    }

    pub fn add(&mut self, theorem: TheoremId<'ctx>, status: ProofStatus<'ctx>) {
        self.theorem_cnt += !status.is_axiom as usize;
        self.axiom_cnt += status.is_axiom as usize;
        self.correct_cnt += status.correct as usize;
        let uses_todo = status.correct && !status.todo_reasons.is_empty();
        self.todo_cnt += uses_todo as usize;
        if uses_todo {
            for reason in &status.todo_reasons {
                *self.todo_by_reason.entry(reason.clone()).or_insert(0) += 1;
            }
        }
        self.statuses.insert(theorem, status);
    }

    pub fn total_cnt(&self) -> usize {
        self.statuses.len()
    }

    pub fn theorem_cnt(&self) -> usize {
        self.theorem_cnt
    }

    pub fn axiom_cnt(&self) -> usize {
        self.axiom_cnt
    }

    pub fn correct_cnt(&self) -> usize {
        self.correct_cnt
    }

    pub fn todo_cnt(&self) -> usize {
        self.todo_cnt
    }

    pub fn todo_by_reason(&self) -> &FxHashMap<Option<String>, usize> {
        &self.todo_by_reason
    }

    pub fn error_cnt(&self) -> usize {
        self.total_cnt() - self.correct_cnt()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TheoremId<'ctx>, &ProofStatus<'ctx>)> {
        self.statuses.iter()
    }
}

impl<'ctx> Index<TheoremId<'ctx>> for ProofStatuses<'ctx> {
    type Output = ProofStatus<'ctx>;

    fn index(&self, index: TheoremId<'ctx>) -> &Self::Output {
        &self.statuses[&index]
    }
}

#[derive(Debug)]
pub struct ProofStatus<'ctx> {
    correct: bool,
    todo_reasons: FxHashSet<Option<String>>,
    is_axiom: bool,
    theorems_used: FxHashSet<TheoremId<'ctx>>,
}

impl<'ctx> ProofStatus<'ctx> {
    pub fn new_axiom() -> Self {
        Self {
            is_axiom: true,
            correct: true,
            todo_reasons: FxHashSet::default(),
            theorems_used: FxHashSet::default(),
        }
    }

    pub fn new_error() -> Self {
        Self {
            is_axiom: false,
            correct: false,
            todo_reasons: FxHashSet::default(),
            theorems_used: FxHashSet::default(),
        }
    }

    pub fn from_cert(cert: ProofCertificate<'ctx>) -> Self {
        Self {
            is_axiom: false,
            correct: !cert.uses_error(),
            todo_reasons: cert.todo_reasons().iter().cloned().collect(),
            theorems_used: cert.theorems_used().iter().copied().collect(),
        }
    }

    pub fn _correct(&self) -> bool {
        self.correct
    }

    pub fn _todo_used(&self) -> bool {
        !self.todo_reasons.is_empty()
    }

    pub fn _is_axiom(&self) -> bool {
        self.is_axiom
    }

    pub fn theorems_used(&self) -> &FxHashSet<TheoremId<'ctx>> {
        &self.theorems_used
    }
}
