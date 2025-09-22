use std::ops::Index;

use rustc_hash::FxHashMap;
use slotmap::{new_key_type, SlotMap};
use ustr::Ustr;
use crate::semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FragPart {
    Fragment(FragmentId),
    Variable(usize), // Debruijn index
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragTemplateRef {
    name: Ustr,
    args: Vec<FragmentId>,
}