use crate::{
    context::arena::NamedArena,
    declare_intern_handle,
    parse::macros::MacroId,
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};
use rustc_hash::{FxHashMap, FxHashSet};
use ustr::Ustr;

pub struct ParseState<'ctx> {
    // Category info
    new_categories: Vec<CategoryId<'ctx>>,
    categories_by_formal_cat: FxHashMap<FormalSyntaxCatId<'ctx>, CategoryId<'ctx>>,

    // Rule info
    all_rules: FxHashSet<RuleId<'ctx>>,
    rules_by_cat: FxHashMap<CategoryId<'ctx>, Vec<RuleId<'ctx>>>,

    // Grammar information
    can_be_empty: FxHashMap<CategoryId<'ctx>, bool>,
    initial_atoms: FxHashMap<CategoryId<'ctx>, FxHashSet<ParseAtomPattern>>,
}

impl<'ctx> ParseState<'ctx> {
    pub fn new() -> Self {
        Self {
            new_categories: Vec::new(),
            categories_by_formal_cat: FxHashMap::default(),
            all_rules: FxHashSet::default(),
            rules_by_cat: FxHashMap::default(),
            can_be_empty: FxHashMap::default(),
            initial_atoms: FxHashMap::default(),
        }
    }

    pub fn use_cat(&mut self, cat: CategoryId<'ctx>) {
        self.rules_by_cat.insert(cat, Vec::new());
        self.can_be_empty.insert(cat, false);
        self.initial_atoms.insert(cat, FxHashSet::default());
        self.new_categories.push(cat);
        if let SyntaxCategorySource::FormalLang(formal) = cat.source() {
            self.categories_by_formal_cat.insert(formal, cat);
        }
    }

    pub fn use_rule(&mut self, rule: RuleId<'ctx>) {
        self.all_rules.insert(rule);
        self.rules_by_cat.get_mut(&rule.cat()).unwrap().push(rule);
        self.recompute_initial_atoms();
    }

    fn recompute_initial_atoms(&mut self) {
        // The first step it to compute which categories have empty rules.
        let mut changed = true;
        while changed {
            changed = false;
            for rule in &self.all_rules {
                if !self.can_be_empty[&rule.cat()]
                    && rule.pattern().parts().iter().all(|part| match part {
                        RulePatternPart::Atom(_) => false,
                        RulePatternPart::Cat { id, .. } => self.can_be_empty[id],
                    })
                {
                    self.can_be_empty.insert(rule.cat(), true);
                    changed = true;
                }
            }
        }

        // Next we compute the initial atoms for each category.
        let mut changed = true;
        while changed {
            changed = false;
            for &rule in &self.all_rules {
                for part in rule.0.pattern().parts() {
                    match part {
                        RulePatternPart::Atom(atom) => {
                            if self
                                .initial_atoms
                                .get_mut(&rule.cat())
                                .unwrap()
                                .insert(*atom)
                            {
                                changed = true;
                            }
                            break;
                        }
                        RulePatternPart::Cat { id, .. } => {
                            if *id != rule.cat() {
                                let [from, to] =
                                    self.initial_atoms.get_disjoint_mut([id, &rule.cat()]);
                                let from = from.unwrap();
                                let to = to.unwrap();

                                for &initial in from.iter() {
                                    if to.insert(initial) {
                                        changed = true;
                                    }
                                }
                            }

                            if !self.can_be_empty[&id] {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn rules_for_cat(&self, cat: CategoryId<'ctx>) -> &[RuleId<'ctx>] {
        &self.rules_by_cat[&cat]
    }

    pub fn cat_for_formal_cat(&self, formal_cat: FormalSyntaxCatId<'ctx>) -> CategoryId<'ctx> {
        self.categories_by_formal_cat[&formal_cat]
    }

    pub fn initial_atoms(&self, cat: CategoryId<'ctx>) -> &FxHashSet<ParseAtomPattern> {
        &self.initial_atoms[&cat]
    }

    pub fn can_be_empty(&self, cat: CategoryId<'ctx>) -> bool {
        self.can_be_empty[&cat]
    }

    pub fn pop_new_categories(&mut self) -> Option<CategoryId<'ctx>> {
        self.new_categories.pop()
    }
}

pub struct ParseRules<'ctx> {
    categories: NamedArena<Category<'ctx>, CategoryId<'ctx>>,
    rules: NamedArena<Rule<'ctx>, RuleId<'ctx>>,
}

impl<'ctx> ParseRules<'ctx> {
    pub fn new() -> Self {
        Self {
            categories: NamedArena::new(),
            rules: NamedArena::new(),
        }
    }

    fn add_cat(&'ctx self, cat: Category<'ctx>) -> CategoryId<'ctx> {
        assert!(self.categories.get(cat.name).is_none());
        self.categories.alloc(cat.name, cat)
    }

    pub fn add_builtin_cat(&'ctx self, name: impl AsRef<str>) -> CategoryId<'ctx> {
        let cat = Category {
            name: name.as_ref().into(),
            source: SyntaxCategorySource::Builtin,
        };
        self.add_cat(cat)
    }

    pub fn add_formal_lang_cat(
        &'ctx self,
        name: Ustr,
        source: FormalSyntaxCatId<'ctx>,
    ) -> CategoryId<'ctx> {
        let cat = Category {
            name,
            source: SyntaxCategorySource::FormalLang(source),
        };
        self.add_cat(cat)
    }

    pub fn add_rule(&'ctx self, rule: Rule<'ctx>) -> RuleId<'ctx> {
        self.rules.alloc(rule.name, rule)
    }

    pub fn cat_by_name(&self, name: Ustr) -> Option<CategoryId<'ctx>> {
        self.categories.get(name)
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

    pub fn source(&self) -> SyntaxCategorySource<'ctx> {
        self.source
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
    pub fn _new(level: usize) -> Self {
        Self(level)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Associativity {
    _Left,
    _Right,
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

    pub fn parts(&self) -> &[RulePatternPart<'ctx>] {
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
