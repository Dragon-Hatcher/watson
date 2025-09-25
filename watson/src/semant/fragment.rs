use std::ops::Index;

use crate::{
    context::Ctx,
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
        theorems::Fact,
    },
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
    bindings_added: usize,
}

impl FragRuleApplication {
    pub fn new(rule: FormalSyntaxRuleId, children: Vec<FragPart>, bindings_added: usize) -> Self {
        Self {
            rule,
            children,
            bindings_added,
        }
    }

    pub fn rule(&self) -> FormalSyntaxRuleId {
        self.rule
    }

    pub fn children(&self) -> &[FragPart] {
        &self.children
    }

    pub fn bindings_added(&self) -> usize {
        self.bindings_added
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

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn args(&self) -> &[FragmentId] {
        &self.args
    }
}

pub fn _debug_fact(fact: Fact, ctx: &Ctx) -> String {
    if let Some(assumption) = fact.assumption() {
        format!(
            "assume {} |- {}",
            _debug_fragment(assumption, ctx),
            _debug_fragment(fact.conclusion(), ctx)
        )
    } else {
        _debug_fragment(fact.conclusion(), ctx)
    }
}

pub fn _debug_fragment(frag: FragmentId, ctx: &Ctx) -> String {
    fn recurse(frag: FragmentId, ctx: &Ctx, mut bound_count: usize) -> String {
        fn print_part(part: &FragPart, ctx: &Ctx, bound_count: usize) -> String {
            match part {
                FragPart::Fragment(frag) => recurse(*frag, ctx, bound_count),
                FragPart::Variable(_cat, idx) => {
                    format!("?{}", bound_count - idx - 1)
                }
            }
        }

        let frag = &ctx.fragments[frag];

        match &frag.data {
            FragData::Rule(rule_app) => {
                let rule = &ctx.formal_syntax[rule_app.rule];
                let mut str = String::new();
                let mut child_idx = 0;

                if rule.pattern().parts().len() > 1 {
                    str.push('(');
                }

                let first_bind = bound_count;
                let mut bind_offset = 0;

                for part in rule.pattern().parts() {
                    if let FormalSyntaxPatPart::Binding(_) = part {
                        bound_count += 1;
                    }
                }

                for part in rule.pattern().parts() {
                    match part {
                        FormalSyntaxPatPart::Binding(_) => {
                            str.push_str(&format!("?{}", first_bind + bind_offset));
                            str.push(' ');
                            bind_offset += 1;
                        }
                        FormalSyntaxPatPart::Cat(_) | FormalSyntaxPatPart::Var(_) => {
                            let part = print_part(&rule_app.children[child_idx], ctx, bound_count);
                            str.push_str(&part);
                            str.push(' ');

                            child_idx += 1;
                        }
                        FormalSyntaxPatPart::Lit(lit) => {
                            str.push_str(lit.as_str());
                            str.push(' ');
                        }
                    }
                }

                if str.ends_with(' ') {
                    str.pop();
                }

                if rule.pattern().parts().len() > 1 {
                    str.push(')');
                }

                str
            }
            FragData::Template(template) => {
                if template.args.is_empty() {
                    format!("{}", template.name)
                } else {
                    format!(
                        "{}({})",
                        template.name,
                        template
                            .args
                            .iter()
                            .map(|arg| recurse(*arg, ctx, bound_count))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            FragData::Hole(idx) => format!("_{}", idx),
        }
    }

    recurse(frag, ctx, 0)
}
