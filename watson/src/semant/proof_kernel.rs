use crate::{
    context::Ctx,
    semant::{
        fragment::{Fact, FragHead, Fragment, FragmentId},
        proof_kernel::safe::{SafeFact, SafeFrag},
        theorems::TheoremId,
    },
};
use itertools::Itertools;

pub struct ProofCertificate<'ctx> {
    proof: ProofState<'ctx>,
}

impl<'ctx> ProofCertificate<'ctx> {
    fn new(proof: ProofState<'ctx>, ctx: &Ctx<'ctx>) -> Result<Self, ProofError> {
        let conclusion = proof.theorem.conclusion();
        let conclusion = SafeFrag::new(conclusion.frag(), ctx)?;

        if !proof
            .knowns
            .contains(&SafeFact::new_conclusion_safe(conclusion))
        {
            return Err(ProofError::ProofIncomplete);
        }

        if !proof.assumptions.is_empty() {
            return Err(ProofError::StillHasAssumptions);
        }

        Ok(ProofCertificate { proof })
    }

    pub fn theorems_used(&self) -> &im::HashSet<TheoremId<'ctx>> {
        &self.proof.theorems_used
    }

    pub fn uses_todo(&self) -> bool {
        self.proof.uses_todo
    }

    pub fn uses_error(&self) -> bool {
        self.proof.uses_error
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofState<'ctx> {
    /// The theorem this is a proof of.
    theorem: TheoremId<'ctx>,
    /// Theorems that were invoked to create the proof
    theorems_used: im::HashSet<TheoremId<'ctx>>,
    /// Facts that are known given all the assumptions
    knowns: im::HashSet<SafeFact<'ctx>>,
    /// Stack of assumptions and the set of known facts before the assumption.
    assumptions: im::Vector<(im::HashSet<SafeFact<'ctx>>, SafeFrag<'ctx>)>,

    uses_todo: bool,
    uses_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProofError {
    FragNotSentence,
    FragHasHoles,
    FragHasVarHoles,
    FragUnclosed,
    NoAssumption,
    ProofIncomplete,
    StillHasAssumptions,
    MissingHypothesis,
}

mod safe {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SafeFrag<'ctx>(FragmentId<'ctx>);

    impl<'ctx> SafeFrag<'ctx> {
        pub fn new(frag: FragmentId<'ctx>, ctx: &Ctx<'ctx>) -> Result<Self, ProofError> {
            if frag.cat() != ctx.sentence_cat {
                Err(ProofError::FragNotSentence)
            } else if frag.has_hole() {
                Err(ProofError::FragHasHoles)
            } else if frag.has_var_hole() {
                Err(ProofError::FragHasVarHoles)
            } else {
                Ok(Self(frag))
            }
        }

        pub fn frag(&self) -> FragmentId<'ctx> {
            self.0
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SafeFact<'ctx> {
        assumption: Option<SafeFrag<'ctx>>,
        conclusion: SafeFrag<'ctx>,
    }

    impl<'ctx> SafeFact<'ctx> {
        pub fn new(fact: Fact<'ctx>, ctx: &Ctx<'ctx>) -> Result<Self, ProofError> {
            let conclusion = SafeFrag::new(fact.conclusion(), ctx)?;
            let assumption = match fact.assumption() {
                Some(assumption) => Some(SafeFrag::new(assumption, ctx)?),
                None => None,
            };
            Ok(Self {
                assumption,
                conclusion,
            })
        }

        pub fn new_conclusion_safe(conclusion: SafeFrag<'ctx>) -> Self {
            Self {
                assumption: None,
                conclusion,
            }
        }

        pub fn _fact(&self) -> Fact<'ctx> {
            Fact::new(self.assumption.map(|a| a.frag()), self.conclusion.frag())
        }
    }
}

impl<'ctx> ProofState<'ctx> {
    pub fn new_from_theorem(theorem: TheoremId<'ctx>, ctx: &Ctx<'ctx>) -> Result<Self, ProofError> {
        let knowns: Result<im::HashSet<_>, _> = theorem
            .hypotheses()
            .iter()
            .map(|h| SafeFact::new(h.fact(), ctx))
            .collect();
        let knowns = knowns?;
        Ok(Self {
            assumptions: im::Vector::new(),
            knowns,
            theorem,
            theorems_used: im::HashSet::new(),
            uses_todo: false,
            uses_error: false,
        })
    }

    pub fn add_assumption(
        &self,
        assumption: FragmentId<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> Result<Self, ProofError> {
        let assumption = SafeFrag::new(assumption, ctx)?;

        let mut new = self.clone();
        let mut new_knowns = new.knowns.clone();
        new_knowns.insert(SafeFact::new_conclusion_safe(assumption));
        let old_knowns = new.knowns;
        new.assumptions.push_back((old_knowns, assumption));
        new.knowns = new_knowns;
        Ok(new)
    }

    pub fn pop_assumption(
        &self,
        justifying: FragmentId<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> Result<Self, ProofError> {
        let mut new = self.clone();
        let (mut old_knowns, assumption) =
            new.assumptions.pop_back().ok_or(ProofError::NoAssumption)?;
        let new_fact = Fact::new(Some(assumption.frag()), justifying);
        let new_fact = SafeFact::new(new_fact, ctx)?;
        old_knowns.insert(new_fact);
        new.knowns = old_knowns;
        Ok(new)
    }

    pub fn apply_theorem(
        &self,
        theorem: TheoremId<'ctx>,
        templates: &[FragmentId<'ctx>],
        ctx: &Ctx<'ctx>,
    ) -> Result<Self, ProofError> {
        let hypotheses: Result<Vec<_>, _> = theorem
            .hypotheses()
            .iter()
            .map(|h| instantiate_fact(h.fact(), templates, ctx))
            .map(|h| SafeFact::new(h, ctx))
            .collect();
        let hypotheses = hypotheses?;

        for hypothesis in hypotheses {
            if !self.knowns.contains(&hypothesis) {
                return Err(ProofError::MissingHypothesis);
            }
        }

        let conclusion = instantiate_frag(theorem.conclusion().frag(), templates, ctx);
        let conclusion = SafeFrag::new(conclusion, ctx)?;
        let mut new = self.clone();
        new.knowns.insert(SafeFact::new_conclusion_safe(conclusion));
        new.theorems_used.insert(theorem);
        Ok(new)
    }

    pub fn apply_todo(
        &self,
        justifying: FragmentId<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> Result<Self, ProofError> {
        let mut new = self.clone();
        let new_fact = Fact::new(None, justifying);
        let new_fact = SafeFact::new(new_fact, ctx)?;
        new.knowns.insert(new_fact);
        new.uses_todo = true;
        Ok(new)
    }

    pub fn apply_error(
        &self,
        justifying: FragmentId<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> Result<Self, ProofError> {
        let mut new = self.clone();
        let new_fact = Fact::new(None, justifying);
        let new_fact = SafeFact::new(new_fact, ctx)?;
        new.knowns.insert(new_fact);
        new.uses_error = true;
        Ok(new)
    }

    pub fn complete(&self, ctx: &Ctx<'ctx>) -> Result<ProofCertificate<'ctx>, ProofError> {
        ProofCertificate::new(self.clone(), ctx)
    }
}

impl<'ctx> ProofState<'ctx> {
    pub fn theorem(&self) -> TheoremId<'ctx> {
        self.theorem
    }
}

fn instantiate_fact<'ctx>(
    fact: Fact<'ctx>,
    templates: &[FragmentId<'ctx>],
    ctx: &Ctx<'ctx>,
) -> Fact<'ctx> {
    let assumption = fact
        .assumption()
        .map(|a| instantiate_frag(a, templates, ctx));
    let conclusion = instantiate_frag(fact.conclusion(), templates, ctx);
    Fact::new(assumption, conclusion)
}

fn instantiate_frag<'ctx>(
    frag: FragmentId<'ctx>,
    templates: &[FragmentId<'ctx>],
    ctx: &Ctx<'ctx>,
) -> FragmentId<'ctx> {
    if !frag.has_template() {
        return frag;
    }

    match frag.head() {
        FragHead::RuleApplication(_) => {
            let new_children = frag
                .children()
                .iter()
                .map(|&c| instantiate_frag(c, templates, ctx))
                .collect();
            let frag = Fragment::new(frag.cat(), frag.head(), new_children);
            ctx.arenas.fragments.intern(frag)
        }
        FragHead::Hole(_) => frag,
        FragHead::VarHole(_) => todo!(),
        FragHead::Var(_) => todo!(),
        FragHead::TemplateRef(idx) => {
            let new_children = frag
                .children()
                .iter()
                .map(|&c| instantiate_frag(c, templates, ctx))
                .collect_vec();
            fill_holes(templates[idx], &new_children, ctx)
        }
    }
}

fn fill_holes<'ctx>(
    frag: FragmentId<'ctx>,
    children: &[FragmentId<'ctx>],
    ctx: &Ctx<'ctx>,
) -> FragmentId<'ctx> {
    if !frag.has_hole() {
        return frag;
    }

    match frag.head() {
        FragHead::Hole(idx) => children[idx],
        _ => {
            let new_children = frag
                .children()
                .iter()
                .map(|&c| fill_holes(c, children, ctx))
                .collect();
            let frag = Fragment::new(frag.cat(), frag.head(), new_children);
            ctx.arenas.fragments.intern(frag)
        }
    }
}
