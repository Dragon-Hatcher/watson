use crate::semant::tactic::syntax::{CustomGrammarCatId, CustomGrammarRuleId};
use rustc_hash::FxHashMap;
use ustr::Ustr;

pub struct CustomGrammarManager<'ctx> {
    all_cats: Vec<CustomGrammarCatId<'ctx>>,
    cats_by_name: FxHashMap<Ustr, CustomGrammarCatId<'ctx>>,
    cats_by_lua_name: FxHashMap<Ustr, CustomGrammarCatId<'ctx>>,
    rules_by_name: FxHashMap<Ustr, CustomGrammarRuleId<'ctx>>,
    rules_by_cat: FxHashMap<CustomGrammarCatId<'ctx>, Vec<CustomGrammarRuleId<'ctx>>>,
}

impl<'ctx> CustomGrammarManager<'ctx> {
    pub fn new() -> Self {
        Self {
            all_cats: Vec::new(),
            cats_by_name: FxHashMap::default(),
            cats_by_lua_name: FxHashMap::default(),
            rules_by_name: FxHashMap::default(),
            rules_by_cat: FxHashMap::default(),
        }
    }

    pub fn use_cat(&mut self, cat: CustomGrammarCatId<'ctx>) {
        self.all_cats.push(cat);
        self.cats_by_name.insert(cat.name(), cat);
        self.cats_by_lua_name.insert(cat.lua_name(), cat);
        self.rules_by_cat.insert(cat, Vec::new());
    }

    pub fn use_rule(&mut self, rule: CustomGrammarRuleId<'ctx>) {
        self.rules_by_name.insert(rule.name(), rule);
        self.rules_by_cat.entry(rule.cat()).or_default().push(rule);
    }

    pub fn cats(&self) -> &[CustomGrammarCatId<'ctx>] {
        &self.all_cats
    }

    pub fn rules_for_cat(&self, cat: CustomGrammarCatId<'ctx>) -> &[CustomGrammarRuleId<'ctx>] {
        &self.rules_by_cat[&cat]
    }

    pub fn rule_by_name(&self, name: Ustr) -> Option<CustomGrammarRuleId<'ctx>> {
        self.rules_by_name.get(&name).copied()
    }
}
