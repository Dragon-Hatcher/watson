use std::ops::Index;

use crate::semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use ustr::Ustr;

pub struct FragmentForest {
    fragments: SlotMap<FragmentId, Fragment>,
    ids_by_fragment: FxHashMap<Fragment, FragmentId>,
}

impl FragmentForest {
    pub fn new() -> Self {
        Self {
            fragments: SlotMap::default(),
            ids_by_fragment: FxHashMap::default(),
        }
    }

    pub fn get_or_insert(&mut self, frag: Fragment) -> FragmentId {
        if let Some(&id) = self.ids_by_fragment.get(&frag) {
            id
        } else {
            let id = self.fragments.insert(frag.clone());
            self.ids_by_fragment.insert(frag, id);
            id
        }
    }
}

impl Index<FragmentId> for FragmentForest {
    type Output = Fragment;

    fn index(&self, index: FragmentId) -> &Self::Output {
        &self.fragments[index]
    }
}

new_key_type! { pub struct FragmentId; }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fragment {
    cat: FormalSyntaxCatId,
    data: FragData,
}

impl Fragment {
    pub fn new(cat: FormalSyntaxCatId, data: FragData) -> Self {
        Self { cat, data }
    }

    pub fn cat(&self) -> FormalSyntaxCatId {
        self.cat
    }

    pub fn data(&self) -> &FragData {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FragData {
    Rule(FragRuleApplication),
    Template(FragTemplateRef),
    Hole(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragRuleApplication {
    rule: FormalSyntaxRuleId,
    children: Vec<FragPart>,
}

impl FragRuleApplication {
    pub fn new(rule: FormalSyntaxRuleId, children: Vec<FragPart>) -> Self {
        Self { rule, children }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FragPart {
    Fragment(FragmentId),
    Variable(FormalSyntaxCatId, usize), // Debruijn index
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragTemplateRef {
    name: Ustr,
    args: Vec<FragmentId>,
}
