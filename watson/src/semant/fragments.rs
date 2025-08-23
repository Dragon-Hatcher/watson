use crate::semant::{
    formal_syntax::{FormalSyntax, FormalSyntaxCatId, FormalSyntaxPatternPart, FormalSyntaxRuleId},
    theorem::{Fact, Template},
    unresolved::{UnresolvedFact, UnresolvedFragment, UnresolvedFragmentData},
};
use itertools::Itertools;
use slotmap::SlotMap;
use std::collections::HashMap;
use ustr::Ustr;

#[derive(Debug)]
pub struct FragCtx {
    frags: SlotMap<FragId, Frag>,
    frags_to_id: HashMap<Frag, FragId>,
}

impl FragCtx {
    pub fn new() -> Self {
        Self {
            frags: SlotMap::default(),
            frags_to_id: HashMap::new(),
        }
    }

    pub fn get(&self, id: FragId) -> &Frag {
        &self.frags[id]
    }

    pub fn get_or_insert(&mut self, frag: Frag) -> FragId {
        *self
            .frags_to_id
            .entry(frag.clone())
            .or_insert_with(|| self.frags.insert(frag))
    }
}

slotmap::new_key_type! { pub struct FragId; }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frag {
    cat: FormalSyntaxCatId,
    data: FragData,
}

impl Frag {
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
    Rule {
        rule: FormalSyntaxRuleId,
        bindings: usize,
        parts: Vec<FragPart>,
    },
    Template {
        name: Ustr,
        args: Vec<FragId>,
    },
    TemplateArgHole(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FragPart {
    Var(usize), // De Bruijn index
    Frag(FragId),
}

pub fn resolve_frag(
    unresolved: UnresolvedFragment,
    templates: &HashMap<Ustr, &Template>,
    shorthands: &HashMap<Ustr, FragId>,
    bindings: &mut Vec<(Ustr, FormalSyntaxCatId)>,
    allow_holes: bool,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
) -> FragId {
    use UnresolvedFragmentData as UnFragDat;

    let frag = match unresolved.data {
        UnFragDat::FormalRule {
            formal_cat,
            formal_rule,
            children,
            ..
        } => {
            let mut bindings_added = 0;
            for child in &children {
                if let UnFragDat::Binding { name, cat } = child.data {
                    bindings.push((name, cat));
                    bindings_added += 1;
                }
            }

            let mut parts = Vec::new();
            let pat_parts = formal.get_rule(formal_rule).pat().parts();

            for (child, pat) in children.into_iter().zip(pat_parts) {
                let part = match child.data {
                    UnFragDat::Binding { .. } | UnFragDat::Lit(_) => continue,
                    UnFragDat::FormalRule { .. } => FragPart::Frag(resolve_frag(
                        child,
                        templates,
                        shorthands,
                        bindings,
                        allow_holes,
                        formal,
                        ctx,
                    )),
                    UnFragDat::VarOrTemplate { name, .. } => match pat {
                        FormalSyntaxPatternPart::Cat(_) => FragPart::Frag(resolve_frag(
                            child,
                            templates,
                            shorthands,
                            bindings,
                            allow_holes,
                            formal,
                            ctx,
                        )),
                        FormalSyntaxPatternPart::Variable(var_cat) => {
                            if let Some((idx, (_, b_cat))) = bindings
                                .iter()
                                .rev()
                                .find_position(|(b_name, _)| *b_name == name)
                            {
                                if b_cat != var_cat {
                                    todo!("err");
                                }

                                FragPart::Var(idx)
                            } else {
                                todo!("err")
                            }
                        }
                        _ => unreachable!(),
                    },
                };
                parts.push(part);
            }

            for _ in 0..bindings_added {
                bindings.pop();
            }

            Frag {
                cat: formal_cat,
                data: FragData::Rule {
                    rule: formal_rule,
                    bindings: bindings_added,
                    parts,
                },
            }
        }
        UnFragDat::VarOrTemplate {
            formal_cat,
            name,
            args,
        } => {
            if let Some(idx) = is_template_arg_hole_name(name) {
                if !args.is_empty() {
                    todo!("err: template arg hole with args");
                }

                if !allow_holes {
                    todo!("err: template arg hole not allowed");
                }

                return ctx.get_or_insert(Frag {
                    cat: formal_cat,
                    data: FragData::TemplateArgHole(idx),
                });
            // Check if this is really a variable.
            } else if args.is_empty()
                && let Some(solo_rule) = formal.solo_var_rule(formal_cat)
                && let Some((pos, (_b_name, b_cat))) = bindings
                    .iter()
                    .rev()
                    .find_position(|(b_name, _)| *b_name == name)
            {
                if *b_cat != formal_cat {
                    todo!("err: mismatched cat");
                }

                Frag {
                    cat: formal_cat,
                    data: FragData::Rule {
                        rule: solo_rule,
                        bindings: 0,
                        parts: vec![FragPart::Var(pos)],
                    },
                }

            // Or if it is a shorthand.
            } else if args.is_empty()
                && let Some(replacement) = shorthands.get(&name)
            {
                let shorthand_cat = ctx.get(*replacement).cat;
                if shorthand_cat != formal_cat {
                    todo!("err: mismatched cat");
                }

                return *replacement;

            // Ok, it really is a template.
            } else if let Some(template) = templates.get(&name) {
                if template.cat() != formal_cat {
                    todo!("err: mismatched cat");
                }

                if template.params().len() != args.len() {
                    todo!("err: mismatched args len")
                }

                let mut arg_frags = Vec::new();
                for (param, arg) in template.params().iter().zip(args.into_iter()) {
                    match arg.data {
                        UnFragDat::FormalRule { formal_cat, .. }
                        | UnFragDat::VarOrTemplate { formal_cat, .. } => {
                            if formal_cat != *param {
                                todo!("err: mismatched cat");
                            }

                            let arg_frag_id = resolve_frag(
                                arg,
                                templates,
                                shorthands,
                                bindings,
                                allow_holes,
                                formal,
                                ctx,
                            );
                            arg_frags.push(arg_frag_id);
                        }
                        UnFragDat::Binding { .. } | UnFragDat::Lit(_) => unreachable!(),
                    }
                }

                Frag {
                    cat: formal_cat,
                    data: FragData::Template {
                        name,
                        args: arg_frags,
                    },
                }
            } else {
                dbg!(name);
                todo!("err: no match for name")
            }
        }
        // We are assuming this node is actually a syntax category in the formal
        // language, not just an atom.
        UnFragDat::Binding { .. } | UnFragDat::Lit(_) => unreachable!(),
    };
    ctx.get_or_insert(frag)
}

pub fn is_template_arg_hole_name(name: Ustr) -> Option<usize> {
    if let Some(idx) = name.as_str().strip_prefix('_') {
        if idx.is_empty() {
            Some(0)
        } else {
            idx.parse().ok()
        }
    } else {
        None
    }
}

pub fn resolve_fact(
    fact: UnresolvedFact,
    templates: &HashMap<Ustr, &Template>,
    shorthands: &HashMap<Ustr, FragId>,
    bindings: &mut Vec<(Ustr, FormalSyntaxCatId)>,
    allow_holes: bool,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
) -> Fact {
    Fact::new(
        fact.assumption.map(|u_frag| {
            resolve_frag(
                u_frag,
                templates,
                shorthands,
                bindings,
                allow_holes,
                formal,
                ctx,
            )
        }),
        resolve_frag(
            fact.statement,
            templates,
            shorthands,
            bindings,
            allow_holes,
            formal,
            ctx,
        ),
    )
}

pub fn _debug_fragment(frag: FragId, ctx: &FragCtx) -> String {
    let frag = ctx.get(frag);
    match &frag.data {
        FragData::Rule {
            rule,
            bindings,
            parts,
        } => format!(
            "Rule({}, {}, [{}])",
            rule.name(),
            bindings,
            parts
                .iter()
                .map(|part| match part {
                    FragPart::Var(idx) => format!("Var({idx})"),
                    FragPart::Frag(frag) => _debug_fragment(*frag, ctx),
                })
                .join(", ")
        ),
        FragData::Template { name, args } => format!(
            "Template({}, [{}])",
            name,
            args.iter()
                .map(|arg| _debug_fragment(*arg, ctx))
                .join(", ")
        ),
        FragData::TemplateArgHole(idx) => format!("TemplateArgHole({idx})"),
    }
}