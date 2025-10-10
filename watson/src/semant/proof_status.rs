use std::ops::Index;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::semant::theorems::TheoremId;

#[derive(Debug)]
pub struct ProofStatuses<'ctx> {
    statuses: FxHashMap<TheoremId<'ctx>, ProofStatus<'ctx>>,
    theorem_cnt: usize,
    axiom_cnt: usize,
    correct_cnt: usize,
    todo_cnt: usize,
}

impl<'ctx> ProofStatuses<'ctx> {
    pub fn new() -> Self {
        Self {
            statuses: FxHashMap::default(),
            theorem_cnt: 0,
            axiom_cnt: 0,
            correct_cnt: 0,
            todo_cnt: 0,
        }
    }

    pub fn add(&mut self, theorem: TheoremId<'ctx>, status: ProofStatus<'ctx>) {
        self.theorem_cnt += !status.is_axiom as usize;
        self.axiom_cnt += status.is_axiom as usize;
        self.correct_cnt += status.correct as usize;
        self.todo_cnt += (status.correct && status.todo_used) as usize;
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
    todo_used: bool,
    is_axiom: bool,
    theorems_used: FxHashSet<TheoremId<'ctx>>,
}

impl<'ctx> ProofStatus<'ctx> {
    pub fn new_axiom() -> Self {
        Self {
            correct: true,
            todo_used: false,
            is_axiom: true,
            theorems_used: FxHashSet::default(),
        }
    }

    pub fn new_theorem(
        correct: bool,
        todo_used: bool,
        theorems_used: FxHashSet<TheoremId<'ctx>>,
    ) -> Self {
        Self {
            correct,
            todo_used,
            is_axiom: false,
            theorems_used,
        }
    }

    pub fn _correct(&self) -> bool {
        self.correct
    }

    pub fn _todo_used(&self) -> bool {
        self.todo_used
    }

    pub fn _is_axiom(&self) -> bool {
        self.is_axiom
    }

    pub fn theorems_used(&self) -> &FxHashSet<TheoremId<'ctx>> {
        &self.theorems_used
    }
}
