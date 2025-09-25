use std::ops::Index;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::semant::{check_circularity::find_circular_dependency_groups, theorems::TheoremId};

#[derive(Debug)]
pub struct ProofStatuses {
    statuses: FxHashMap<TheoremId, ProofStatus>,
    circulars: Vec<Vec<TheoremId>>,
    theorem_cnt: usize,
    axiom_cnt: usize,
    correct_cnt: usize,
    todo_cnt: usize,
}

impl ProofStatuses {
    pub fn new() -> Self {
        Self {
            statuses: FxHashMap::default(),
            circulars: Vec::new(),
            theorem_cnt: 0,
            axiom_cnt: 0,
            correct_cnt: 0,
            todo_cnt: 0,
        }
    }

    pub fn add(&mut self, theorem: TheoremId, status: ProofStatus) {
        self.theorem_cnt += !status.is_axiom as usize;
        self.axiom_cnt += status.is_axiom as usize;
        self.correct_cnt += status.correct as usize;
        self.todo_cnt += status.todo_used as usize;
        self.statuses.insert(theorem, status);
    }

    pub fn recompute_circular_dependencies(&mut self) {
        self.circulars = find_circular_dependency_groups(self);
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

    pub fn iter(&self) -> impl Iterator<Item = (&TheoremId, &ProofStatus)> {
        self.statuses.iter()
    }

    pub fn circular_dependencies(&self) -> &[Vec<TheoremId>] {
        &self.circulars
    }
}

impl Index<TheoremId> for ProofStatuses {
    type Output = ProofStatus;

    fn index(&self, index: TheoremId) -> &Self::Output {
        &self.statuses[&index]
    }
}

#[derive(Debug)]
pub struct ProofStatus {
    correct: bool,
    todo_used: bool,
    is_axiom: bool,
    theorems_used: FxHashSet<TheoremId>,
}

impl ProofStatus {
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
        theorems_used: FxHashSet<TheoremId>,
    ) -> Self {
        Self {
            correct,
            todo_used,
            is_axiom: false,
            theorems_used,
        }
    }

    pub fn correct(&self) -> bool {
        self.correct
    }

    pub fn todo_used(&self) -> bool {
        self.todo_used
    }

    pub fn is_axiom(&self) -> bool {
        self.is_axiom
    }

    pub fn theorems_used(&self) -> &FxHashSet<TheoremId> {
        &self.theorems_used
    }
}
