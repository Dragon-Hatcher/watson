use crate::semant::{
    parse_fragment::{UnresolvedAnyFrag, UnresolvedFact, UnresolvedFrag},
    tactic::syntax::TacticRuleId,
};
use ustr::Ustr;

pub enum UnresolvedProof<'ctx> {
    Axiom,
    Theorem(TacticInst<'ctx>),
}

pub struct TacticInst<'ctx> {
    rule: TacticRuleId<'ctx>,
    children: Vec<TacticInstPart<'ctx>>,
}

impl<'ctx> TacticInst<'ctx> {
    pub fn new(rule: TacticRuleId<'ctx>, children: Vec<TacticInstPart<'ctx>>) -> Self {
        Self { rule, children }
    }

    pub fn rule(&self) -> TacticRuleId<'ctx> {
        self.rule
    }

    pub fn children(&self) -> &[TacticInstPart<'ctx>] {
        &self.children
    }
}

pub enum TacticInstPart<'ctx> {
    NoInstantiation,
    Name(Ustr),
    SubInst(TacticInst<'ctx>),
    Frag(UnresolvedFrag<'ctx>),
    AnyFrag(UnresolvedAnyFrag<'ctx>),
    Fact(UnresolvedFact<'ctx>),
}
