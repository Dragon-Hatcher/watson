use crate::{
    parse::{Span, parse_tree::ParseRuleId},
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TheoremId(Ustr);

impl TheoremId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

#[derive(Debug, Clone)]
pub struct UnresolvedTheorem {
    id: TheoremId,
    templates: Vec<Template>,
    hypotheses: Vec<UnresolvedFact>,
    conclusion: UnresolvedFragment,
    proof: Proof,
}

impl UnresolvedTheorem {
    pub fn new(
        id: TheoremId,
        templates: Vec<Template>,
        hypotheses: Vec<UnresolvedFact>,
        conclusion: UnresolvedFragment,
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

#[derive(Debug, Clone)]
pub struct UnresolvedFragment {
    pub span: Span,
    pub data: UnresolvedFragmentData,
}

#[derive(Debug, Clone)]
pub enum UnresolvedFragmentData {
    Binding(Ustr),
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
