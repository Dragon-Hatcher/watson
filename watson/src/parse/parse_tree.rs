use crate::{
    context::Ctx,
    parse::{
        Span,
        parse_state::{CategoryId, ParseAtomPattern, RuleId},
    },
};
use rustc_hash::FxHashMap;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use std::ops::Index;
use ustr::Ustr;

pub struct ParseForest {
    trees: SlotMap<ParseTreeId, ParseTree>,
    ids_by_tree: FxHashMap<ParseTree, ParseTreeId>,
    has_unexpanded_macro: SecondaryMap<ParseTreeId, bool>,
}

impl ParseForest {
    pub fn new() -> Self {
        Self {
            trees: SlotMap::default(),
            ids_by_tree: FxHashMap::default(),
            has_unexpanded_macro: SecondaryMap::default(),
        }
    }

    fn check_has_unexpanded_macro(&self, tree: &ParseTree) -> bool {
        for possibility in &tree.possibilities {
            for part in &possibility.children {
                match part {
                    ParseTreePart::Atom(atom) => {
                        if let ParseAtomKind::MacroBinding(_) = atom.kind() {
                            return true;
                        }
                    }
                    ParseTreePart::Node { id, .. } => {
                        if self.has_unexpanded_macro[*id] {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn get_or_insert(&mut self, tree: ParseTree) -> ParseTreeId {
        if let Some(&id) = self.ids_by_tree.get(&tree) {
            id
        } else {
            let has_unexpanded_macro = self.check_has_unexpanded_macro(&tree);
            let id = self.trees.insert(tree.clone());
            self.ids_by_tree.insert(tree, id);
            self.has_unexpanded_macro[id] = has_unexpanded_macro;
            id
        }
    }

    pub fn has_unexpanded_macro(&self, tree: ParseTreeId) -> bool {
        self.has_unexpanded_macro[tree]
    }
}

impl Index<ParseTreeId> for ParseForest {
    type Output = ParseTree;

    fn index(&self, index: ParseTreeId) -> &Self::Output {
        &self.trees[index]
    }
}

new_key_type! { pub struct ParseTreeId; }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseTree {
    span: Span,
    cat: CategoryId,
    possibilities: Vec<ParseTreeChildren>,
}

impl ParseTree {
    pub fn new(span: Span, cat: CategoryId, possibilities: Vec<ParseTreeChildren>) -> Self {
        Self {
            span,
            cat,
            possibilities,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn cat(&self) -> CategoryId {
        self.cat
    }

    pub fn possibilities(&self) -> &[ParseTreeChildren] {
        &self.possibilities
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseTreeChildren {
    rule: RuleId,
    children: Vec<ParseTreePart>,
}

impl ParseTreeChildren {
    pub fn new(rule: RuleId, children: Vec<ParseTreePart>) -> Self {
        Self { rule, children }
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
    Node {
        id: ParseTreeId,
        span: Span,
        cat: CategoryId,
    },
}

impl ParseTreePart {
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

    pub fn as_node(&self) -> Option<ParseTreeId> {
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

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn kind(&self) -> ParseAtomKind {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseMacroBinding {
    name: Ustr,
    span: Span,
    pat: ParseAtomPattern,
}

pub fn _debug_parse_tree(tree: ParseTreeId, ctx: &Ctx) {
    fn recurse(tree: ParseTreeId, ctx: &Ctx, indent: usize) {
        let tree = &ctx.parse_forest[tree];
        let indent_str = "│ ".repeat(indent);

        let possibilities = &tree.possibilities;
        if possibilities.len() == 1 {
            let possibility = &possibilities[0];

            println!(
                "{}ParseTree (cat: {:?}, rule: {:?}, span: {:?})",
                indent_str,
                ctx.parse_state[tree.cat()].name(),
                ctx.parse_state[possibility.rule()].name(),
                tree.span()
            );

            for child in possibility.children() {
                match child {
                    ParseTreePart::Atom(atom) => {
                        println!(
                            "{}│ Atom (kind: {:?}, span: {:?})",
                            indent_str,
                            atom.kind(),
                            atom.span()
                        );
                    }
                    ParseTreePart::Node { id, .. } => {
                        recurse(*id, ctx, indent + 1);
                    }
                }
            }

            if possibility.children().is_empty() {
                println!("{indent_str}  <empty>");
            }
        } else {
            println!(
                "{}ParseTree (cat: {:?}, span: {:?}) !AMBIGUOUS!",
                indent_str,
                ctx.parse_state[tree.cat()].name(),
                tree.span()
            );

            for (i, possibility) in possibilities.iter().enumerate() {
                println!(
                    "{} • Possibility {} (rule: {:?})",
                    indent_str,
                    i + 1,
                    ctx.parse_state[possibility.rule()].name()
                );
                for child in possibility.children() {
                    match child {
                        ParseTreePart::Atom(atom) => {
                            println!(
                                "{}    Atom (kind: {:?}, span: {:?})",
                                indent_str,
                                atom.kind(),
                                atom.span()
                            );
                        }
                        ParseTreePart::Node { id, .. } => {
                            recurse(*id, ctx, indent + 2);
                        }
                    }
                }

                if possibility.children().is_empty() {
                    println!("{indent_str}    <empty>");
                }
            }

            if possibilities.is_empty() {
                println!("{indent_str}    !!!!!!! POSSIBILITIES EMPTY");
            }
        }
    }

    recurse(tree, ctx, 0);
}
