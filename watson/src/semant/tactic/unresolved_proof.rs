use crate::{
    parse::Span,
    semant::{
        parse_fragment::{UnresolvedAnyFrag, UnresolvedFact, UnresolvedFrag},
        tactic::syntax::CustomGrammarRuleId,
    },
};
use mlua::FromLua;
use ustr::Ustr;

pub enum UnresolvedProof<'ctx> {
    Axiom,
    Theorem(TacticInst<'ctx>),
}

pub struct TacticInst<'ctx> {
    rule: CustomGrammarRuleId<'ctx>,
    span: Span,
    children: Vec<TacticInstPart<'ctx>>,
}

impl<'ctx> TacticInst<'ctx> {
    pub fn new(
        rule: CustomGrammarRuleId<'ctx>,
        span: Span,
        children: Vec<TacticInstPart<'ctx>>,
    ) -> Self {
        Self {
            rule,
            span,
            children,
        }
    }

    pub fn rule(&self) -> CustomGrammarRuleId<'ctx> {
        self.rule
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn children(&self) -> &[TacticInstPart<'ctx>] {
        &self.children
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct SpannedStr {
    str: Ustr,
    span: Span,
}

impl SpannedStr {
    pub fn new(str: Ustr, span: Span) -> Self {
        Self { str, span }
    }

    pub fn str(&self) -> Ustr {
        self.str
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

pub enum TacticInstPart<'ctx> {
    Kw(SpannedStr),
    Lit(SpannedStr),
    Name(SpannedStr),
    SubInst(TacticInst<'ctx>),
    Frag(UnresolvedFrag<'ctx>),
    AnyFrag(UnresolvedAnyFrag<'ctx>),
    Fact(UnresolvedFact<'ctx>),
}
