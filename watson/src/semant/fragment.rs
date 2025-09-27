use std::ops::Index;

use crate::{
    context::{Ctx, arena::InternedArena},
    declare_intern_handle,
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
        theorems::Fact,
    },
};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use ustr::Ustr;

pub struct FragmentForest<'ctx> {
    fragments: InternedArena<Fragment<'ctx>, FragmentId<'ctx>>,
}

impl<'ctx> FragmentForest<'ctx> {
    pub fn new() -> Self {
        Self {
            fragments: InternedArena::new(),
        }
    }

    pub fn get_or_insert(&'ctx self, frag: Fragment<'ctx>) -> FragmentId<'ctx> {
        self.fragments.intern(frag)
    }
}

declare_intern_handle! { FragmentId<'ctx> => Fragment<'ctx> }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fragment<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    data: FragData<'ctx>,
}

impl<'ctx> Fragment<'ctx> {
    pub fn new(cat: FormalSyntaxCatId<'ctx>, data: FragData<'ctx>) -> Self {
        Self { cat, data }
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn data(&'ctx self) -> &'ctx FragData<'ctx> {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FragData<'ctx> {
    Rule(FragRuleApplication<'ctx>),
    Template(FragTemplateRef<'ctx>),
    Hole(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragRuleApplication<'ctx> {
    rule: FormalSyntaxRuleId<'ctx>,
    children: Vec<FragPart<'ctx>>,
    bindings_added: usize,
}

impl<'ctx> FragRuleApplication<'ctx> {
    pub fn new(
        rule: FormalSyntaxRuleId<'ctx>,
        children: Vec<FragPart<'ctx>>,
        bindings_added: usize,
    ) -> Self {
        Self {
            rule,
            children,
            bindings_added,
        }
    }

    pub fn rule(&self) -> FormalSyntaxRuleId<'ctx> {
        self.rule
    }

    pub fn children(&self) -> &[FragPart<'ctx>] {
        &self.children
    }

    pub fn bindings_added(&self) -> usize {
        self.bindings_added
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FragPart<'ctx> {
    Fragment(FragmentId<'ctx>),
    Variable(FormalSyntaxCatId<'ctx>, usize), // Debruijn index
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragTemplateRef<'ctx> {
    name: Ustr,
    args: Vec<FragmentId<'ctx>>,
}

impl<'ctx> FragTemplateRef<'ctx> {
    pub fn new(name: Ustr, args: Vec<FragmentId<'ctx>>) -> Self {
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

        match &frag.data {
            FragData::Rule(rule_app) => {
                let mut str = String::new();
                let mut child_idx = 0;

                if rule_app.rule.pattern().parts().len() > 1 {
                    str.push('(');
                }

                let first_bind = bound_count;
                let mut bind_offset = 0;

                for part in rule_app.rule.pattern().parts() {
                    if let FormalSyntaxPatPart::Binding(_) = part {
                        bound_count += 1;
                    }
                }

                for part in rule_app.rule.pattern().parts() {
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

                if rule_app.rule.pattern().parts().len() > 1 {
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
