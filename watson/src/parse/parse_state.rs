use crate::{
    parse::macros::MacroId,
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};
use rustc_hash::FxHashMap;
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::ops::Index;
use ustr::Ustr;

pub struct ParseState {
    categories: SlotMap<CategoryId, Category>,
    categories_by_name: FxHashMap<Ustr, CategoryId>,
    new_categories: Vec<CategoryId>,

    rules: SlotMap<RuleId, Rule>,
    rules_by_cat: SecondaryMap<CategoryId, Vec<RuleId>>,
}

impl ParseState {
    pub fn new() -> Self {
        Self {
            categories: SlotMap::default(),
            categories_by_name: FxHashMap::default(),
            new_categories: Vec::new(),
            rules: SlotMap::default(),
            rules_by_cat: SecondaryMap::default(),
        }
    }

    fn add_cat(&mut self, cat: Category) -> CategoryId {
        let name = cat.name;
        assert!(!self.categories_by_name.contains_key(&name));
        let id = self.categories.insert(cat);
        self.categories_by_name.insert(name, id);
        self.rules_by_cat.insert(id, Vec::new());
        self.new_categories.push(id);
        id
    }

    pub fn new_builtin_cat(&mut self, name: impl AsRef<str>) -> CategoryId {
        let cat = Category {
            name: name.as_ref().into(),
            source: SyntaxCategorySource::Builtin,
        };
        self.add_cat(cat)
    }

    pub fn pop_new_categories(&mut self) -> Option<CategoryId> {
        self.new_categories.pop()
    }

    pub fn add_rule(&mut self, rule: Rule) -> RuleId {
        let cat = rule.cat;
        let id = self.rules.insert(rule);
        self.rules_by_cat[cat].push(id);
        id
    }

    pub fn rules_for_cat(&self, cat: CategoryId) -> &[RuleId] {
        &self.rules_by_cat[cat]
    }
}

impl Index<CategoryId> for ParseState {
    type Output = Category;

    fn index(&self, index: CategoryId) -> &Self::Output {
        &self.categories[index]
    }
}

impl Index<RuleId> for ParseState {
    type Output = Rule;

    fn index(&self, index: RuleId) -> &Self::Output {
        &self.rules[index]
    }
}

new_key_type! { pub struct CategoryId; }
new_key_type! { pub struct RuleId; }

pub struct Category {
    name: Ustr,
    source: SyntaxCategorySource,
}

impl Category {
    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn source(&self) -> &SyntaxCategorySource {
        &self.source
    }
}

pub enum SyntaxCategorySource {
    Builtin,
    FormalLang(FormalSyntaxCatId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Precedence(usize);

impl Precedence {
    pub fn new(level: usize) -> Self {
        Self(level)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Associativity {
    Left,
    Right,
    NonAssoc,
}

pub struct Rule {
    name: Ustr,
    cat: CategoryId,
    source: ParseRuleSource,
    pattern: RulePattern,
    precedence: Precedence,
    associativity: Associativity,
}

impl Rule {
    pub fn new(
        name: impl AsRef<str>,
        cat: CategoryId,
        source: ParseRuleSource,
        pattern: RulePattern,
    ) -> Self {
        Self {
            name: name.as_ref().into(),
            cat,
            source,
            pattern,
            precedence: Precedence(0),
            associativity: Associativity::NonAssoc,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> CategoryId {
        self.cat
    }

    pub fn source(&self) -> &ParseRuleSource {
        &self.source
    }

    pub fn pattern(&self) -> &RulePattern {
        &self.pattern
    }

    pub fn precedence(&self) -> Precedence {
        self.precedence
    }

    pub fn associativity(&self) -> Associativity {
        self.associativity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseRuleSource {
    Builtin,
    FormalLang(FormalSyntaxRuleId),
    Macro(MacroId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RulePattern {
    parts: Vec<RulePatternPart>,
}

impl RulePattern {
    pub fn new(parts: Vec<RulePatternPart>) -> Self {
        Self { parts }
    }

    pub fn parts(&self) -> &[RulePatternPart] {
        &self.parts
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RulePatternPart {
    Atom(ParseAtomPattern),
    Cat(CategoryId),
    TempCat(CategoryId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseAtomPattern {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Str,
}
