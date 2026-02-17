use crate::{
    parse::Span,
    semant::{
        custom_grammar::syntax::CustomGrammarRuleId,
        parse_fragment::{UnresolvedAnyFrag, UnresolvedFact, UnresolvedFrag},
    },
};
use mlua::FromLua;
use ustr::Ustr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGrammarInst<'ctx> {
    rule: CustomGrammarRuleId<'ctx>,
    span: Span,
    children: Vec<CustomGrammarInstPart<'ctx>>,
}

impl<'ctx> CustomGrammarInst<'ctx> {
    pub fn new(
        rule: CustomGrammarRuleId<'ctx>,
        span: Span,
        children: Vec<CustomGrammarInstPart<'ctx>>,
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

    pub fn children(&self) -> &[CustomGrammarInstPart<'ctx>] {
        &self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromLua)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomGrammarInstPart<'ctx> {
    Kw(SpannedStr),
    Lit(SpannedStr),
    Name(SpannedStr),
    SubInst(CustomGrammarInst<'ctx>),
    Frag(UnresolvedFrag<'ctx>),
    AnyFrag(UnresolvedAnyFrag<'ctx>),
    Fact(UnresolvedFact<'ctx>),
}
