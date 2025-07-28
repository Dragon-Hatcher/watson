use crate::parse::Span;
use ustr::Ustr;

macro_rules! category_id {
    ($name:ident = $str:literal) => {
        pub static $name: std::sync::LazyLock<crate::parse::parse_tree::SyntaxCategoryId> =
            std::sync::LazyLock::new(|| {
                crate::parse::parse_tree::SyntaxCategoryId::Builtin(ustr::Ustr::from($str))
            });
    };
}

macro_rules! rule_id {
    ($name:ident = $str:literal) => {
        pub static $name: std::sync::LazyLock<crate::parse::parse_tree::ParseRuleId> =
            std::sync::LazyLock::new(|| {
                crate::parse::parse_tree::ParseRuleId::Builtin(ustr::Ustr::from($str))
            });
    };
}

category_id!(COMMAND_CAT = "command");
rule_id!(MODULE_RULE = "module");
rule_id!(MACRO_RULE = "macro");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseTree {
    Atom(ParseAtom),
    Node(ParseNode),
    Missing(Span),
}

impl ParseTree {
    pub fn span(&self) -> Span {
        match self {
            ParseTree::Atom(parse_atom) => parse_atom.full_span,
            ParseTree::Node(parse_node) => parse_node.span,
            ParseTree::Missing(span) => *span,
        }
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, ParseTree::Missing(_))
    }

    pub fn is_atom_kind(&self, kind: ParseAtomKind) -> bool {
        match self {
            ParseTree::Atom(got) => got.kind == kind,
            _ => false,
        }
    }

    pub fn is_kw(&self, str: Ustr) -> bool {
        self.is_atom_kind(ParseAtomKind::Kw(str))
    }

    pub fn as_name(&self) -> Option<Ustr> {
        match self {
            ParseTree::Atom(ParseAtom {
                kind: ParseAtomKind::Name(name),
                ..
            }) => Some(*name),
            _ => None,
        }
    }

    pub fn as_rule(&self, rule: ParseRuleId) -> Option<&[ParseTree]> {
        if let Self::Node(n) = self
            && n.rule == rule
        {
            Some(&n.children)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNode {
    category: SyntaxCategoryId,
    rule: ParseRuleId,
    children: Vec<ParseTree>,
    span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxCategoryId {
    Builtin(Ustr),
    UserDef(),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseAtom {
    pub full_span: Span,
    pub content_span: Span,
    pub kind: ParseAtomKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseAtomKind {
    Lit(Ustr),
    Kw(Ustr),
    Name(Ustr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseRuleId {
    Builtin(Ustr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRule {
    pub id: ParseRuleId,
    pub cat: SyntaxCategoryId,
    pub pattern: Vec<PatternPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternPart {
    Atom(AtomPattern),
    Category(SyntaxCategoryId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomPattern {
    Lit(Ustr),
    Kw(Ustr),
    Name,
}
