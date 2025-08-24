use crate::{
    diagnostics::{DiagManager, WResult},
    semant::{
        check_proofs::ProofState,
        formal_syntax::{FormalSyntax, FormalSyntaxCatId, FormalSyntaxRuleId},
        theorem::{Fact, Template, TheoremId},
        unresolved::{
            UnresolvedFact, UnresolvedFragPart, UnresolvedFragment, UnresolvedFragmentData,
        },
    },
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

#[allow(clippy::too_many_arguments)]
pub fn resolve_frag(
    unresolved: UnresolvedFragment,
    templates: &HashMap<Ustr, &Template>,
    shorthands: &HashMap<Ustr, FragId>,
    bindings: &mut Vec<(Ustr, FormalSyntaxCatId)>,
    allow_holes: bool,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
    diags: &mut DiagManager,
    in_theorem: TheoremId,
    proof_state: Option<&ProofState>,
) -> WResult<FragId> {
    use UnresolvedFragmentData as UnFragDat;

    let formal_cat = unresolved.formal_cat;
    let frag = match unresolved.data {
        UnFragDat::FormalRule {
            formal_rule,
            children,
            ..
        } => {
            let mut bindings_added = 0;
            for child in &children {
                if let &UnresolvedFragPart::Binding { name, cat } = child {
                    bindings.push((name, cat));
                    bindings_added += 1;
                }
            }

            let mut parts = Vec::new();

            for child in children {
                let part = match child {
                    UnresolvedFragPart::Binding { .. } | UnresolvedFragPart::Lit => continue,
                    UnresolvedFragPart::Frag(frag) => FragPart::Frag(resolve_frag(
                        frag,
                        templates,
                        shorthands,
                        bindings,
                        allow_holes,
                        formal,
                        ctx,
                        diags,
                        in_theorem,
                        proof_state,
                    )?),
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
        UnFragDat::VarOrTemplate { name, args } => {
            if let Some(idx) = is_template_arg_hole_name(name) {
                if !args.is_empty() {
                    todo!("err: template arg hole with args");
                }

                if !allow_holes {
                    todo!("err: template arg hole not allowed");
                }

                return Ok(ctx.get_or_insert(Frag {
                    cat: formal_cat,
                    data: FragData::TemplateArgHole(idx),
                }));
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

                return Ok(*replacement);

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
                    if arg.formal_cat != *param {
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
                        diags,
                        in_theorem,
                        proof_state,
                    )?;
                    arg_frags.push(arg_frag_id);
                }

                Frag {
                    cat: formal_cat,
                    data: FragData::Template {
                        name,
                        args: arg_frags,
                    },
                }
            } else {
                diags.err_unknown_name(in_theorem, proof_state.cloned(), name, unresolved.span);

                return Err(());
            }
        }
    };
    Ok(ctx.get_or_insert(frag))
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

#[allow(clippy::too_many_arguments)]
pub fn resolve_fact(
    fact: UnresolvedFact,
    templates: &HashMap<Ustr, &Template>,
    shorthands: &HashMap<Ustr, FragId>,
    bindings: &mut Vec<(Ustr, FormalSyntaxCatId)>,
    allow_holes: bool,
    formal: &FormalSyntax,
    ctx: &mut FragCtx,
    diags: &mut DiagManager,
    in_theorem: TheoremId,
    proof_state: Option<&ProofState>,
) -> WResult<Fact> {
    Ok(Fact::new(
        fact.assumption
            .map(|u_frag| {
                resolve_frag(
                    u_frag,
                    templates,
                    shorthands,
                    bindings,
                    allow_holes,
                    formal,
                    ctx,
                    diags,
                    in_theorem,
                    proof_state,
                )
            })
            .transpose()?,
        resolve_frag(
            fact.statement,
            templates,
            shorthands,
            bindings,
            allow_holes,
            formal,
            ctx,
            diags,
            in_theorem,
            proof_state,
        )?,
    ))
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
            rule._name(),
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
            args.iter().map(|arg| _debug_fragment(*arg, ctx)).join(", ")
        ),
        FragData::TemplateArgHole(idx) => format!("TemplateArgHole({idx})"),
    }
}
