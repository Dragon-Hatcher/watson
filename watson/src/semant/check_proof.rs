use rustc_hash::{FxHashMap, FxHashSet};
use std::vec;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{Span, elaborator::elaborate_tactic, parse_tree::ParseTreeId},
    semant::{
        fragment::{
            FragData, FragPart, FragRuleApplication, FragTemplateRef, Fragment, FragmentId,
        },
        parse_fragment::{NameCtx, UnresolvedFact, parse_any_fragment, parse_fragment},
        proof_status::ProofStatus,
        theorems::{Fact, TheoremId, TheoremStatement, UnresolvedProof},
    },
};

pub fn check_proofs(ctx: &mut Ctx) {
    let theorem_ids: Vec<TheoremId> = ctx.theorem_stmts.iter().map(|(id, _)| *id).collect();

    for id in theorem_ids {
        let Ok(status) = check_proof(id, ctx) else {
            continue;
        };

        ctx.proof_statuses.add(id, status);
    }
}

#[derive(Debug, Clone)]
struct ProofState {
    knowns: FxHashSet<Fact>,
    goal: FragmentId,
    names: NameCtx,
}

fn check_proof(id: TheoremId, ctx: &mut Ctx) -> WResult<ProofStatus> {
    let theorem_smt = &ctx.theorem_stmts[id];

    let proof = match theorem_smt.proof() {
        UnresolvedProof::Axiom => return Ok(ProofStatus::new_axiom()),
        UnresolvedProof::Theorem(proof) => *proof,
    };
    let start_state = ProofState {
        knowns: theorem_smt.hypotheses().iter().cloned().collect(),
        goal: theorem_smt.conclusion(),
        names: name_ctx_from_smt(theorem_smt),
    };

    let mut proof_states = vec![(start_state, proof)];
    let mut proof_correct = true;
    let mut todo_used = false;
    let mut theorems_used = FxHashSet::default();

    while let Some((mut state, proof)) = proof_states.pop() {
        let tactic = elaborate_tactic(proof, ctx)?;

        match tactic {
            UnresolvedTactic::None => {
                let goal_fact = Fact::new(None, state.goal);
                if !state.knowns.contains(&goal_fact) {
                    proof_correct = false;
                }
            }
            UnresolvedTactic::Have(tactic) => {
                let mut sub_state = state.clone();

                // Add the assumption to the sub-state if there is one.
                let assumption = if let Some(assumption) = tactic.fact.assumption() {
                    let Ok(assumption) = parse_fragment(
                        assumption,
                        ctx.formal_syntax.sentence_cat(),
                        &mut sub_state.names,
                        ctx,
                    ) else {
                        // There isn't much we can do if the assumption doesn't parse
                        // we just drop this state and the continuation too.
                        proof_correct = false;
                        continue;
                    };

                    let assumption_fact = Fact::new(None, assumption);
                    sub_state.knowns.insert(assumption_fact);
                    Some(assumption)
                } else {
                    None
                };

                let Ok(conclusion) = parse_fragment(
                    tactic.fact.conclusion(),
                    ctx.formal_syntax.sentence_cat(),
                    &mut state.names,
                    ctx,
                ) else {
                    proof_correct = false;
                    continue;
                };
                sub_state.goal = conclusion;

                proof_states.push((sub_state, tactic.proof));

                let conclusion_fact = Fact::new(assumption, conclusion);
                state.knowns.insert(conclusion_fact);
                proof_states.push((state, tactic.continuation));
            }
            UnresolvedTactic::By(tactic) => {
                let theorem_id = TheoremId::new(tactic.theorem_name);
                let Some(theorem) = ctx.theorem_stmts.get(theorem_id) else {
                    // The theorem doesn't exist.
                    proof_correct = false;
                    ctx.diags
                        .err_non_existent_theorem(tactic.theorem_name, tactic.theorem_name_span);
                    continue;
                };

                theorems_used.insert(theorem_id);

                if theorem.templates().len() != tactic.templates.len() {
                    // The number of templates doesn't match.
                    proof_correct = false;
                    if theorem.templates().len() > tactic.templates.len() {
                        ctx.diags.err_missing_tactic_templates(
                            tactic
                                .templates
                                .last()
                                .map(|t| ctx.parse_forest[*t].span())
                                .unwrap_or(tactic.theorem_name_span),
                            theorem.templates().len() - tactic.templates.len(),
                        );
                    } else {
                        ctx.diags.err_extra_tactic_templates(
                            ctx.parse_forest[tactic.templates[theorem.templates().len()]].span(),
                            tactic.templates.len() - theorem.templates().len(),
                        );
                    }
                    continue;
                }

                let mut template_instantiations = Vec::new();
                for (template, instantiation) in theorem
                    .templates()
                    .to_vec()
                    .iter()
                    .zip(tactic.templates.iter())
                {
                    for &param_cat in template.params() {
                        state.names.add_hole(param_cat)
                    }

                    let Ok(instantiation) =
                        parse_any_fragment(*instantiation, template.cat(), &mut state.names, ctx)
                    else {
                        proof_correct = false;
                        continue;
                    };

                    state.names.clear_holes();
                    template_instantiations.push(instantiation);
                }

                let theorem = &ctx.theorem_stmts[theorem_id];

                if template_instantiations.len() != theorem.templates().len() {
                    // One of the template instantiations had an error.
                    continue;
                }

                let mut templates = FxHashMap::default();
                for (i, template) in theorem.templates().iter().enumerate() {
                    templates.insert(template.name(), template_instantiations[i]);
                }

                for hypothesis in theorem.hypotheses().to_vec() {
                    let instantiated = instantiate_fact_with_templates(hypothesis, &templates, ctx);

                    if !state.knowns.contains(&instantiated) {
                        proof_correct = false;
                        // TODO: error message.
                    }
                }

                let theorem = &ctx.theorem_stmts[theorem_id];
                let conclusion =
                    instantiate_fragment_with_templates(theorem.conclusion(), &templates, ctx);

                if conclusion != state.goal {
                    proof_correct = false;
                    // TODO: error message.
                }
            }
            UnresolvedTactic::Todo => {
                todo_used = true;
            }
        }
    }

    Ok(ProofStatus::new_theorem(
        proof_correct,
        todo_used,
        theorems_used,
    ))
}

fn instantiate_fact_with_templates(
    fact: Fact,
    templates: &FxHashMap<Ustr, FragmentId>,
    ctx: &mut Ctx,
) -> Fact {
    let assumption = fact
        .assumption()
        .map(|assump| instantiate_fragment_with_templates(assump, templates, ctx));
    let conclusion = instantiate_fragment_with_templates(fact.conclusion(), templates, ctx);
    Fact::new(assumption, conclusion)
}

fn instantiate_fragment_with_templates(
    frag: FragmentId,
    templates: &FxHashMap<Ustr, FragmentId>,
    ctx: &mut Ctx,
) -> FragmentId {
    let frag = &ctx.fragments[frag];
    let cat = frag.cat();

    match frag.data() {
        FragData::Rule(rule) => {
            let formal_rule = rule.rule();
            let bindings_added = rule.bindings_added();

            let mut new_parts = Vec::new();
            for part in rule.children().to_vec() {
                let new_part = match part {
                    FragPart::Fragment(frag_id) => {
                        let new_frag = instantiate_fragment_with_templates(frag_id, templates, ctx);
                        FragPart::Fragment(new_frag)
                    }
                    FragPart::Variable(cat, idx) => FragPart::Variable(cat, idx),
                };
                new_parts.push(new_part);
            }
            let data = FragData::Rule(FragRuleApplication::new(
                formal_rule,
                new_parts,
                bindings_added,
            ));
            ctx.fragments.get_or_insert(Fragment::new(cat, data))
        }
        FragData::Template(temp) => {
            let replacement = templates[&temp.name()];
            let args = temp.args().to_vec();
            fill_template_holes(replacement, &args, 0, ctx)
        }
        FragData::Hole(_) => unreachable!(),
    }
}

fn fill_template_holes(
    frag_id: FragmentId,
    args: &[FragmentId],
    debruijn_shift: usize,
    ctx: &mut Ctx,
) -> FragmentId {
    let frag = &ctx.fragments[frag_id];
    let cat = frag.cat();

    match frag.data().clone() {
        FragData::Rule(rule_app) => {
            let new_children = rule_app
                .children()
                .iter()
                .map(|part| match part {
                    FragPart::Fragment(child_id) => FragPart::Fragment(fill_template_holes(
                        *child_id,
                        args,
                        debruijn_shift + rule_app.bindings_added(),
                        ctx,
                    )),
                    FragPart::Variable(_, _) => *part,
                })
                .collect();
            let data = FragData::Rule(FragRuleApplication::new(
                rule_app.rule(),
                new_children,
                rule_app.bindings_added(),
            ));
            ctx.fragments.get_or_insert(Fragment::new(cat, data))
        }
        FragData::Template(template) => {
            let new_args = template
                .args()
                .iter()
                .map(|arg| fill_template_holes(*arg, args, debruijn_shift, ctx))
                .collect();
            let data = FragData::Template(FragTemplateRef::new(template.name(), new_args));
            ctx.fragments.get_or_insert(Fragment::new(cat, data))
        }
        FragData::Hole(idx) => {
            let replacement = args[idx];
            fix_debruijn_indices(replacement, debruijn_shift, ctx)
        }
    }
}

fn fix_debruijn_indices(frag_id: FragmentId, shift: usize, ctx: &mut Ctx) -> FragmentId {
    if shift == 0 {
        return frag_id;
    }

    let frag = &ctx.fragments[frag_id];
    let cat = frag.cat();

    match frag.data().clone() {
        FragData::Rule(rule_app) => {
            let new_children = rule_app
                .children()
                .iter()
                .map(|part| match part {
                    FragPart::Fragment(child_id) => {
                        FragPart::Fragment(fix_debruijn_indices(*child_id, shift, ctx))
                    }
                    FragPart::Variable(cat, idx) => FragPart::Variable(*cat, *idx + shift),
                })
                .collect();
            let data = FragData::Rule(FragRuleApplication::new(
                rule_app.rule(),
                new_children,
                rule_app.bindings_added(),
            ));
            ctx.fragments.get_or_insert(Fragment::new(cat, data))
        }
        FragData::Template(template) => {
            let new_args = template
                .args()
                .iter()
                .map(|&arg| fix_debruijn_indices(arg, shift, ctx))
                .collect();
            let data = FragData::Template(FragTemplateRef::new(template.name(), new_args));
            ctx.fragments.get_or_insert(Fragment::new(cat, data))
        }
        FragData::Hole(_) => frag_id,
    }
}

fn name_ctx_from_smt(smt: &TheoremStatement) -> NameCtx {
    let mut names = NameCtx::new();
    for template in smt.templates() {
        names.add_template(template.name(), template.clone());
    }
    names
}

#[derive(Debug, Clone)]
pub enum UnresolvedTactic {
    None,
    Have(UnresolvedHaveTactic),
    By(UnresolvedByTactic),
    Todo,
}

#[derive(Debug, Clone, Copy)]
pub struct UnresolvedHaveTactic {
    pub fact: UnresolvedFact,
    pub proof: ParseTreeId,
    pub continuation: ParseTreeId,
}

#[derive(Debug, Clone)]
pub struct UnresolvedByTactic {
    pub theorem_name: Ustr,
    pub theorem_name_span: Span,
    pub templates: Vec<ParseTreeId>,
}
