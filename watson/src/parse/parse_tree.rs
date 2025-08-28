use crate::parse::{
    Span,
    parse_state::{CategoryId, ParseAtomPattern, RuleId},
};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use std::ops::Index;
use ustr::Ustr;

pub struct ParseForest {
    trees: SlotMap<ParseTreeId, ParseTree>,
    promises: FxHashMap<ParseTreePromise, Vec<RuleId>>,
}

impl ParseForest {
    pub fn new() -> Self {
        Self {
            trees: SlotMap::default(),
            promises: FxHashMap::default(),
        }
    }

    pub fn add_promise(&mut self, promise: ParseTreePromise, rule: RuleId) {
        self.promises.entry(promise).or_default().push(rule);
    }

    pub fn rules_for_promise(&self, promise: ParseTreePromise) -> &[RuleId] {
        self.promises.get(&promise).map(|v| &v[..]).unwrap_or(&[])
    }

    pub fn resolve_promise(&mut self, _promise: ParseTreePromise, _rule: RuleId) -> ParseTreeId {
        dbg!(_promise);
        todo!()
    }
}

impl Index<ParseTreeId> for ParseForest {
    type Output = ParseTree;

    fn index(&self, index: ParseTreeId) -> &Self::Output {
        &self.trees[index]
    }
}

new_key_type! { pub struct ParseTreeId; }

pub struct ParseTree {
    span: Span,
    cat: CategoryId,
    rule: RuleId,
    children: Vec<ParseTreePart>,
}

impl ParseTree {
    pub fn span(&self) -> Span {
        self.span
    }

    pub fn cat(&self) -> CategoryId {
        self.cat
    }

    pub fn rule(&self) -> RuleId {
        self.rule
    }

    pub fn children(&self) -> &[ParseTreePart] {
        &self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseTreePart {
    Atom(ParseAtom),
    MacroBinding(ParseMacroBinding),
    Node(ParseTreePromise),
}

impl ParseTreePart {
    pub fn span(&self) -> Span {
        match self {
            Self::Atom(atom) => atom.span,
            Self::MacroBinding(binding) => binding.span,
            Self::Node(node) => node.span,
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

    pub fn as_name(&self) -> Option<Ustr> {
        if let Self::Atom(atom) = self
            && let ParseAtomKind::Name(text) = atom.kind
        {
            Some(text)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseAtomKind {
    Lit(Ustr),
    Kw(Ustr),
    Name(Ustr),
    StrLit(Ustr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseTreePromise {
    cat: CategoryId,
    span: Span,
}

impl ParseTreePromise {
    pub fn new(cat: CategoryId, span: Span) -> Self {
        Self { cat, span }
    }

    pub fn cat(&self) -> CategoryId {
        self.cat
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseMacroBinding {
    name: Ustr,
    span: Span,
    pat: ParseAtomPattern,
}
