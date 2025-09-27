use crate::{
    context::arena::NamedArena,
    declare_intern_handle,
    parse::macros::MacroId,
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};
use rustc_hash::{FxHashMap, FxHashSet};
use slotmap::{SecondaryMap, SlotMap, new_key_type};
use std::ops::Deref;
use typed_arena::Arena;
use ustr::Ustr;

pub struct ParseState<'ctx> {
    categories: NamedArena<Category<'ctx>, CategoryId<'ctx>>,
    categories_by_formal_cat: FxHashMap<FormalSyntaxCatId<'ctx>, CategoryId<'ctx>>,
    new_categories: Vec<CategoryId<'ctx>>,

    rules: NamedArena<Rule<'ctx>, RuleId<'ctx>>,
    rules_by_cat: SecondaryMap<CategoryId<'ctx>, Vec<RuleId<'ctx>>>,

    can_be_empty: SecondaryMap<CategoryId<'ctx>, bool>,
    initial_atoms: SecondaryMap<CategoryId<'ctx>, FxHashSet<ParseAtomPattern>>,
}

impl<'ctx> ParseState<'ctx> {
    pub fn new() -> Self {
        Self {
            categories: NamedArena::new(),
            categories_by_formal_cat: FxHashMap::default(),
            new_categories: Vec::new(),
            rules: NamedArena::new(),
            rules_by_cat: SecondaryMap::default(),
            can_be_empty: SecondaryMap::default(),
            initial_atoms: SecondaryMap::default(),
        }
    }

    fn add_cat(&mut self, cat: Category) -> CategoryId {
        assert!(self.categories.get(cat.name).is_none());
        let id = self.categories.alloc(cat.name, cat);
        self.rules_by_cat.insert(id, Vec::new());
        self.can_be_empty.insert(id, false);
        self.initial_atoms.insert(id, FxHashSet::default());
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

    pub fn new_formal_lang_cat(&mut self, name: Ustr, source: FormalSyntaxCatId) -> CategoryId {
        let cat = Category {
            name,
            source: SyntaxCategorySource::FormalLang(source),
        };
        let id = self.add_cat(cat);
        self.categories_by_formal_cat.insert(source, id);
        id
    }

    pub fn pop_new_categories(&mut self) -> Option<CategoryId> {
        self.new_categories.pop()
    }

    pub fn add_rule(&'ctx mut self, rule: Rule) -> RuleId {
        let cat = rule.cat;
        let id = RuleId(self.rules.alloc(rule));
        self.rules_by_cat[cat].push(id);
        self.recompute_initial_atoms();
        id
    }

    pub fn rules_for_cat(&self, cat: CategoryId) -> &[RuleId] {
        &self.rules_by_cat[cat]
    }

    pub fn cat_by_name(&self, name: Ustr) -> Option<CategoryId> {
        self.categories_by_name.get(&name).copied()
    }

    pub fn cat_for_formal_cat(&self, formal_cat: FormalSyntaxCatId) -> CategoryId {
        self.categories_by_formal_cat[&formal_cat]
    }

    fn recompute_initial_atoms(&mut self) {
        // The first step it to compute which categories have empty rules.
        let mut changed = true;
        while changed {
            changed = false;
            for (_, rule) in &self.rules {
                if !self.can_be_empty[rule.cat]
                    && rule.pattern().parts().iter().all(|part| match part {
                        RulePatternPart::Atom(_) => false,
                        RulePatternPart::Cat { id, .. } => self.can_be_empty[*id],
                    })
                {
                    self.can_be_empty[rule.cat] = true;
                    changed = true;
                }
            }
        }

        // Next we compute the initial atoms for each category.
        let mut changed = true;
        while changed {
            changed = false;
            for (_, rule) in &self.rules {
                for part in rule.pattern().parts() {
                    match part {
                        RulePatternPart::Atom(atom) => {
                            if self.initial_atoms[rule.cat].insert(*atom) {
                                changed = true;
                            }
                            break;
                        }
                        RulePatternPart::Cat { id, .. } => {
                            let initials = self.initial_atoms[*id].clone();
                            for initial in initials {
                                if self.initial_atoms[rule.cat].insert(initial) {
                                    changed = true;
                                }
                            }

                            if !self.can_be_empty[*id] {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn initial_atoms(&self, cat: CategoryId) -> &FxHashSet<ParseAtomPattern> {
        &self.initial_atoms[cat]
    }

    pub fn can_be_empty(&self, cat: CategoryId) -> bool {
        self.can_be_empty[cat]
    }
}

declare_intern_handle!(CategoryId<'ctx> => Category<'ctx>);

declare_intern_handle!(RuleId<'ctx> => Rule<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Category<'ctx> {
    name: Ustr,
    source: SyntaxCategorySource<'ctx>,
}

impl<'ctx> Category<'ctx> {
    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn source(&self) -> &SyntaxCategorySource {
        &self.source
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxCategorySource<'ctx> {
    Builtin,
    FormalLang(FormalSyntaxCatId<'ctx>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Precedence(pub usize);

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule<'ctx> {
    name: Ustr,
    cat: CategoryId<'ctx>,
    source: ParseRuleSource<'ctx>,
    pattern: RulePattern<'ctx>,
}

impl<'ctx> Rule<'ctx> {
    pub fn new(
        name: impl AsRef<str>,
        cat: CategoryId<'ctx>,
        source: ParseRuleSource<'ctx>,
        pattern: RulePattern<'ctx>,
    ) -> Self {
        Self {
            name: name.as_ref().into(),
            cat,
            source,
            pattern,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> CategoryId<'ctx> {
        self.cat
    }

    pub fn source(&self) -> &ParseRuleSource<'ctx> {
        &self.source
    }

    pub fn pattern(&self) -> &RulePattern<'ctx> {
        &self.pattern
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseRuleSource<'ctx> {
    Builtin,
    FormalLang(FormalSyntaxRuleId<'ctx>),
    Macro(MacroId<'ctx>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RulePattern<'ctx> {
    parts: Vec<RulePatternPart<'ctx>>,
    precedence: Precedence,
    associativity: Associativity,
}

impl<'ctx> RulePattern<'ctx> {
    pub fn new(parts: Vec<RulePatternPart<'ctx>>) -> Self {
        Self {
            parts,
            precedence: Precedence(0),
            associativity: Associativity::NonAssoc,
        }
    }

    pub fn parts(&self) -> &[RulePatternPart] {
        &self.parts
    }

    pub fn precedence(&self) -> Precedence {
        self.precedence
    }

    pub fn associativity(&self) -> Associativity {
        self.associativity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RulePatternPart<'ctx> {
    Atom(ParseAtomPattern),
    Cat {
        id: CategoryId<'ctx>,
        template: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ParseAtomPattern {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Str,
    MacroBinding,
}
