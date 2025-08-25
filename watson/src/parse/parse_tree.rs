use crate::{
    parse::{Span, macros::MacroId},
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use std::ops::Index;
use ustr::Ustr;

pub struct ParseForest {
    trees: SlotMap<ParseTreeId, ParseTree>,
    categories: SlotMap<CategoryId, Category>,
    categories_by_name: FxHashMap<Ustr, CategoryId>,
    rules: SlotMap<RuleId, Rule>,
}

impl ParseForest {
    pub fn new() -> Self {
        Self {
            trees: SlotMap::default(),
            categories: SlotMap::default(),
            categories_by_name: FxHashMap::default(),
            rules: SlotMap::default(),
        }
    }

    fn add_cat(&mut self, cat: Category) -> CategoryId {
        let name = cat.name;
        assert!(!self.categories_by_name.contains_key(&name));
        let id = self.categories.insert(cat);
        self.categories_by_name.insert(name, id);
        id
    }

    pub fn new_builtin_cat(&mut self, name: impl AsRef<str>) -> CategoryId {
        let cat = Category {
            name: name.as_ref().into(),
            source: SyntaxCategorySource::Builtin,
        };
        self.add_cat(cat)
    }

    fn add_rule(&mut self, rule: Rule) -> RuleId {
        let id = self.rules.insert(rule);
        id
    }

    pub fn new_builtin_rule(&mut self, name: impl AsRef<str>, cat: CategoryId) -> RuleId {
        let rule = Rule {
            name: name.as_ref().into(),
            cat,
            source: ParseRuleSource::Builtin,
        };
        self.add_rule(rule)
    }
}

impl Index<ParseTreeId> for ParseForest {
    type Output = ParseTree;

    fn index(&self, index: ParseTreeId) -> &Self::Output {
        &self.trees[index]
    }
}

impl Index<CategoryId> for ParseForest {
    type Output = Category;

    fn index(&self, index: CategoryId) -> &Self::Output {
        &self.categories[index]
    }
}

impl Index<RuleId> for ParseForest {
    type Output = Rule;

    fn index(&self, index: RuleId) -> &Self::Output {
        &self.rules[index]
    }
}

new_key_type! { pub struct ParseTreeId; }
new_key_type! { pub struct CategoryId; }
new_key_type! { pub struct RuleId; }

pub struct ParseTree {
    span: Span,
    cat: CategoryId,
    rule: RuleId,
    children: Vec<ParseTreePart>,
}

pub enum ParseTreePart {
    Atom(ParseAtom),
    MacroBinding(ParseMacroBinding),
    Node(ParseTreePromise),
}

pub struct ParseAtom {
    span: Span,
    text: Ustr,
    kind: ParseAtomKind,
}

pub enum ParseAtomKind {
    Lit,
    Kw,
    Name,
    StrLit,
}

pub struct ParseTreePromise {
    cat: CategoryId,
    span: Span,
}

pub struct ParseMacroBinding {
    name: Ustr,
    span: Span,
    pat: ParseAtomPattern,
}

pub struct Category {
    name: Ustr,
    source: SyntaxCategorySource,
}

pub enum SyntaxCategorySource {
    Builtin,
    FormalLang(FormalSyntaxCatId),
}

pub struct Rule {
    name: Ustr,
    cat: CategoryId,
    source: ParseRuleSource,
}

pub enum ParseRuleSource {
    Builtin,
    FormalLang(FormalSyntaxRuleId),
    Macro(MacroId),
}

pub enum ParseAtomPattern {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Str,
}

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum ParseTree {
//     Atom(ParseAtom),
//     Node(ParseNode),
//     MacroBinding(MacroBindingNode),
// }

// impl ParseTree {
//     pub fn span(&self) -> Span {
//         match self {
//             ParseTree::Atom(parse_atom) => parse_atom.full_span,
//             ParseTree::Node(parse_node) => parse_node.span,
//             ParseTree::MacroBinding(macro_binding) => macro_binding.span,
//         }
//     }

//     pub fn is_atom_kind(&self, kind: ParseAtomKind) -> bool {
//         match self {
//             ParseTree::Atom(got) => got.kind == kind,
//             _ => false,
//         }
//     }

//     pub fn is_kw(&self, str: Ustr) -> bool {
//         self.is_atom_kind(ParseAtomKind::Kw(str))
//     }

//     pub fn is_lit(&self, str: Ustr) -> bool {
//         self.is_atom_kind(ParseAtomKind::Lit(str))
//     }

//     pub fn as_name(&self) -> Option<Ustr> {
//         match self {
//             ParseTree::Atom(ParseAtom {
//                 kind: ParseAtomKind::Name(name),
//                 ..
//             }) => Some(*name),
//             _ => None,
//         }
//     }

//     pub fn as_str(&self) -> Option<Ustr> {
//         match self {
//             ParseTree::Atom(ParseAtom {
//                 kind: ParseAtomKind::Str(text),
//                 ..
//             }) => Some(*text),
//             _ => None,
//         }
//     }

//     pub fn as_rule(&self, rule: ParseRuleId) -> Option<&[ParseTree]> {
//         if let Self::Node(n) = self
//             && n.rule == rule
//         {
//             Some(&n.children)
//         } else {
//             None
//         }
//     }

//     pub fn as_rule_pat(&self, expected_name: Ustr) -> Option<&[ParseTree]> {
//         if let Self::Node(n) = self
//             && let ParseRuleId::Pattern(name, _) = n.rule
//             && name == expected_name
//         {
//             Some(&n.children)
//         } else {
//             None
//         }
//     }

//     pub fn as_atom(&self) -> Option<&ParseAtom> {
//         if let Self::Atom(atom) = self {
//             Some(atom)
//         } else {
//             None
//         }
//     }

//     pub fn as_node(&self) -> Option<&ParseNode> {
//         if let Self::Node(node) = self {
//             Some(node)
//         } else {
//             None
//         }
//     }

//     pub fn has_unchecked_bindings(&self) -> bool {
//         match self {
//             ParseTree::Atom(_) => false,
//             ParseTree::Node(node) => node.has_unchecked_bindings,
//             ParseTree::MacroBinding(binding) => binding.is_unchecked,
//         }
//     }
// }

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct ParseNode {
//     pub category: SyntaxCategoryId,
//     pub rule: ParseRuleId,
//     pub children: Vec<ParseTree>,
//     pub span: Span,
//     pub has_unchecked_bindings: bool,
// }

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct MacroBindingNode {
//     pub name: Ustr,
//     pub kind: MacroBindingKind,
//     pub span: Span,
//     pub is_unchecked: bool,
// }

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum MacroBindingKind {
//     Atom(AtomPattern),
//     Cat(SyntaxCategoryId),
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
// pub enum SyntaxCategoryId {
//     Builtin(Ustr),
//     FormalLang(FormalSyntaxCatId),
// }

// impl SyntaxCategoryId {
//     pub fn name(&self) -> Ustr {
//         match self {
//             SyntaxCategoryId::Builtin(name) => *name,
//             SyntaxCategoryId::FormalLang(id) => id.name(),
//         }
//     }
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub struct ParseAtom {
//     pub full_span: Span,
//     pub content_span: Span,
//     pub kind: ParseAtomKind,
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum ParseAtomKind {
//     Lit(Ustr),
//     Kw(Ustr),
//     Name(Ustr),
//     Str(Ustr),
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub enum ParseRuleId {
//     Builtin(Ustr),
//     Pattern(Ustr, SyntaxCategoryId),
//     FormalLang(FormalSyntaxRuleId),
//     Macro(MacroId),
// }

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct ParseRule {
//     pub id: ParseRuleId,
//     pub cat: SyntaxCategoryId,
//     pub pattern: Vec<PatternPart>,
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum PatternPart {
//     Atom(AtomPattern),
//     Category(SyntaxCategoryId),
//     TemplateCat(SyntaxCategoryId),
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub enum AtomPattern {
//     Lit(Ustr),
//     Kw(Ustr),
//     Name,
//     Str,
// }
