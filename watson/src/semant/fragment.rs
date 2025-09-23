use std::ops::Index;

use crate::{
    context::Ctx,
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};
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

impl FragTemplateRef {
    pub fn new(name: Ustr, args: Vec<FragmentId>) -> Self {
        Self { name, args }
    }
}

pub fn _debug_fragment(frag: FragmentId, ctx: &mut Ctx) {
    fn debug_frag(frag: FragmentId, ctx: &mut Ctx, depth: usize) {
        let frag = &ctx.fragments[frag];
        let indent = "  ".repeat(depth);
        match &frag.data.clone() {
            FragData::Rule(app) => {
                let rule = &ctx.formal_syntax[app.rule];
                println!(
                    "{}Rule: {} -> {}",
                    indent,
                    rule.name(),
                    ctx.formal_syntax[rule.cat()].name()
                );
                for child in &app.children {
                    match child {
                        FragPart::Fragment(child_frag) => {
                            debug_frag(*child_frag, ctx, depth + 1);
                        }
                        FragPart::Variable(cat, idx) => {
                            println!(
                                "{}  Var: {}[{}]",
                                indent,
                                ctx.formal_syntax[*cat].name(),
                                idx
                            );
                        }
                    }
                }
            }
            FragData::Template(template) => {
                println!("{}Template: {}", indent, template.name);
                for arg in &template.args {
                    debug_frag(*arg, ctx, depth + 1);
                }
            }
            FragData::Hole(idx) => {
                println!("{}Hole: {}", indent, idx);
            }
        }
    }

    debug_frag(frag, ctx, 0);
}
