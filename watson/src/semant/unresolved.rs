use crate::{
    parse::{
        Span,
        parse_tree::{ParseRuleId, ParseTree},
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
        theorem::{Template, TheoremId},
    },
};
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct UnresolvedTheorem {
    pub(super) id: TheoremId,
    pub(super) templates: Vec<Template>,
    pub(super) hypotheses: Vec<UnresolvedFact>,
    pub(super) conclusion: UnresolvedFragment,
    pub(super) proof: UnresolvedProof,
}

impl UnresolvedTheorem {
    pub fn new(
        id: TheoremId,
        templates: Vec<Template>,
        hypotheses: Vec<UnresolvedFact>,
        conclusion: UnresolvedFragment,
        proof: UnresolvedProof,
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
pub enum UnresolvedProof {
    Axiom,
    Theorem(ParseTree),
}

#[derive(Debug, Clone)]
pub struct UnresolvedFragment {
    pub span: Span,
    pub formal_cat: FormalSyntaxCatId,
    pub data: UnresolvedFragmentData,
}

#[derive(Debug, Clone)]
pub enum UnresolvedFragmentData {
    FormalRule {
        _syntax_rule: ParseRuleId,
        formal_rule: FormalSyntaxRuleId,
        children: Vec<UnresolvedFragPart>,
    },
    VarOrTemplate {
        name: Ustr,
        args: Vec<UnresolvedFragment>,
    },
}

#[derive(Debug, Clone)]
pub enum UnresolvedFragPart {
    Frag(UnresolvedFragment),
    Lit,
    Binding { name: Ustr, cat: FormalSyntaxCatId },
}

#[derive(Debug, Clone)]
pub struct UnresolvedFact {
    pub assumption: Option<UnresolvedFragment>,
    pub statement: UnresolvedFragment,
}
