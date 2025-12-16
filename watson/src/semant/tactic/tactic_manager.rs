use crate::semant::tactic::syntax::{TacticCatId, TacticRuleId};
use rustc_hash::FxHashMap;
use ustr::Ustr;

pub struct TacticManager<'ctx> {
    all_cats: Vec<TacticCatId<'ctx>>,
    tactic_cats_by_name: FxHashMap<Ustr, TacticCatId<'ctx>>,
    tactic_cats_by_lua_name: FxHashMap<Ustr, TacticCatId<'ctx>>,
    tactic_rules_by_name: FxHashMap<Ustr, TacticRuleId<'ctx>>,
    tactic_rules_by_cat: FxHashMap<TacticCatId<'ctx>, Vec<TacticRuleId<'ctx>>>,
}

impl<'ctx> TacticManager<'ctx> {
    pub fn new() -> Self {
        Self {
            all_cats: Vec::new(),
            tactic_cats_by_name: FxHashMap::default(),
            tactic_cats_by_lua_name: FxHashMap::default(),
            tactic_rules_by_name: FxHashMap::default(),
            tactic_rules_by_cat: FxHashMap::default(),
        }
    }

    pub fn use_tactic_cat(&mut self, cat: TacticCatId<'ctx>) {
        self.all_cats.push(cat);
        self.tactic_cats_by_name.insert(cat.name(), cat);
        self.tactic_cats_by_lua_name.insert(cat.lua_name(), cat);
        self.tactic_rules_by_cat.insert(cat, Vec::new());
    }

    pub fn use_tactic_rule(&mut self, rule: TacticRuleId<'ctx>) {
        self.tactic_rules_by_name.insert(rule.name(), rule);
        self.tactic_rules_by_cat
            .entry(rule.cat())
            .or_default()
            .push(rule);
    }

    pub fn cats(&self) -> &[TacticCatId<'ctx>] {
        &self.all_cats
    }

    pub fn rules_for_cat(&self, cat: TacticCatId<'ctx>) -> &[TacticRuleId<'ctx>] {
        &self.tactic_rules_by_cat[&cat]
    }
}
