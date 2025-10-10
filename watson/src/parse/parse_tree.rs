use crate::{
    generate_arena_handle,
    parse::{
        Span,
        parse_state::{CategoryId, RuleId},
    },
};
use std::hash::Hash;
use ustr::Ustr;

generate_arena_handle!(ParseTreeId<'ctx> => ParseTree<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseTree<'ctx> {
    span: Span,
    cat: CategoryId<'ctx>,
    possibilities: Vec<ParseTreeChildren<'ctx>>,
}

impl<'ctx> ParseTree<'ctx> {
    pub fn new(
        span: Span,
        cat: CategoryId<'ctx>,
        possibilities: Vec<ParseTreeChildren<'ctx>>,
    ) -> Self {
        Self {
            span,
            cat,
            possibilities,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn cat(&self) -> CategoryId<'ctx> {
        self.cat
    }

    pub fn possibilities(&self) -> &[ParseTreeChildren<'ctx>] {
        &self.possibilities
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseTreeChildren<'ctx> {
    rule: RuleId<'ctx>,
    children: Vec<ParseTreePart<'ctx>>,
}

impl<'ctx> ParseTreeChildren<'ctx> {
    pub fn new(rule: RuleId<'ctx>, children: Vec<ParseTreePart<'ctx>>) -> Self {
        Self { rule, children }
    }

    pub fn rule(&self) -> RuleId<'ctx> {
        self.rule
    }

    pub fn children(&self) -> &[ParseTreePart<'ctx>] {
        &self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseTreePart<'ctx> {
    Atom(ParseAtom),
    Node {
        id: ParseTreeId<'ctx>,
        span: Span,
        cat: CategoryId<'ctx>,
    },
}

impl<'ctx> ParseTreePart<'ctx> {
    pub fn span(&self) -> Span {
        match self {
            Self::Atom(atom) => atom.span,
            Self::Node { span, .. } => *span,
        }
    }

    pub fn is_kw(&self, kw: Ustr) -> bool {
        if let Self::Atom(atom) = self
            && let ParseAtomKind::Kw(text) = atom.kind
        {
            text == kw
        } else {
            false
        }
    }

    pub fn is_lit(&self, lit: Ustr) -> bool {
        if let Self::Atom(atom) = self
            && let ParseAtomKind::Lit(text) = atom.kind
        {
            text == lit
        } else {
            false
        }
    }

    pub fn as_name(&self) -> Option<Ustr> {
        if let Self::Atom(atom) = self
            && let ParseAtomKind::Name(text) = atom.kind
        {
            Some(text)
        } else {
            None
        }
    }

    pub fn as_str_lit(&self) -> Option<Ustr> {
        if let Self::Atom(atom) = self
            && let ParseAtomKind::StrLit(text) = atom.kind
        {
            Some(text)
        } else {
            None
        }
    }

    pub fn as_macro_binding(&self) -> Option<Ustr> {
        if let Self::Atom(atom) = self
            && let ParseAtomKind::MacroBinding(text) = atom.kind
        {
            Some(text)
        } else {
            None
        }
    }

    pub fn as_node(&self) -> Option<ParseTreeId<'ctx>> {
        if let Self::Node { id, .. } = self {
            Some(*id)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseAtom {
    span: Span,
    kind: ParseAtomKind,
}

impl ParseAtom {
    pub fn new(span: Span, kind: ParseAtomKind) -> Self {
        Self { span, kind }
    }

    pub fn _span(&self) -> Span {
        self.span
    }

    pub fn _kind(&self) -> ParseAtomKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseAtomKind {
    Lit(Ustr),
    Kw(Ustr),
    Name(Ustr),
    StrLit(Ustr),
    MacroBinding(Ustr),
}
