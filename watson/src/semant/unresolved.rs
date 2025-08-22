use crate::{
    parse::{
        Span,
        parse_tree::{ParseRuleId, ParseTree},
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
        fragments::{FragCtx, FragId},
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
    pub data: UnresolvedFragmentData,
}

#[derive(Debug, Clone)]
pub enum UnresolvedFragmentData {
    Binding {
        name: Ustr,
        cat: FormalSyntaxCatId,
    },
    Lit(Ustr),
    FormalRule {
        syntax_rule: ParseRuleId,
        formal_cat: FormalSyntaxCatId,
        formal_rule: FormalSyntaxRuleId,
        children: Vec<UnresolvedFragment>,
    },
    VarOrTemplate {
        formal_cat: FormalSyntaxCatId,
        name: Ustr,
        args: Vec<UnresolvedFragment>,
    },
}

#[derive(Debug, Clone)]
pub struct UnresolvedFact {
    pub assumption: Option<UnresolvedFragment>,
    pub statement: UnresolvedFragment,
}
