use crate::semant::{
    presentation::PresFrag,
    theorems::{PresFact, TheoremId},
};

#[derive(Debug, Clone)]
pub struct TacticInfo<'ctx> {
    steps: im::Vector<TacticInfoStep<'ctx>>,
    goal: PresFrag<'ctx>,
}

impl<'ctx> TacticInfo<'ctx> {
    pub fn new(thm: TheoremId<'ctx>) -> Self {
        Self {
            steps: thm
                .hypotheses()
                .iter()
                .map(|&h| TacticInfoStep::Hypothesis(h))
                .collect(),
            goal: thm.conclusion(),
        }
    }

    pub fn steps(&self) -> &im::Vector<TacticInfoStep<'ctx>> {
        &self.steps
    }

    pub fn goal(&self) -> PresFrag<'ctx> {
        self.goal
    }

    fn add_step(&self, step: TacticInfoStep<'ctx>) -> Self {
        let mut new_steps = self.steps.clone();
        new_steps.push_back(step);
        Self {
            steps: new_steps,
            goal: self.goal,
        }
    }

    pub fn with_assume(&self, f: PresFrag<'ctx>) -> Self {
        self.add_step(TacticInfoStep::Assume(f))
    }

    pub fn with_deduce(&self, f: PresFact<'ctx>) -> Self {
        self.add_step(TacticInfoStep::Deduce(f))
    }

    pub fn with_goal(&self, f: PresFrag<'ctx>) -> Self {
        Self {
            steps: self.steps.clone(),
            goal: f,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TacticInfoStep<'ctx> {
    Hypothesis(PresFact<'ctx>),
    Assume(PresFrag<'ctx>),
    Deduce(PresFact<'ctx>),
}
