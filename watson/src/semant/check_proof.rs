use rustc_hash::{FxHashMap, FxHashSet};
use std::vec;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    generate_arena_handle,
    parse::{Span, elaborator::elaborate_tactic, parse_tree::ParseTreeId},
    semant::{
        fragment::{
            _debug_fact, _debug_fragment, FragData, FragPart, FragRuleApplication, FragTemplateRef,
            Fragment, FragmentId,
        },
        parse_fragment::{NameCtx, UnresolvedFact, parse_any_fragment, parse_fragment},
        proof_status::{ProofStatus, ProofStatuses},
        theorems::{Fact, Template, TheoremId, UnresolvedProof},
    },
};

pub fn check_proofs<'ctx>(
    theorems: &[TheoremId<'ctx>],
    ctx: &mut Ctx<'ctx>,
) -> ProofStatuses<'ctx> {
    let mut statuses = ProofStatuses::new();
    for &id in theorems {
        let Ok(status) = check_proof(id, ctx) else {
            continue;
        };

        statuses.add(id, status);
    }

    statuses
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofState<'ctx> {
    knowns: FxHashSet<Fact<'ctx>>,
    goal: FragmentId<'ctx>,
    templates: FxHashMap<Ustr, Template<'ctx>>,
    shorthands: FxHashMap<Ustr, FragmentId<'ctx>>,

    reasoning_chain: Vec<ReasoningStep<'ctx>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningStep<'ctx> {
    Hypothesis(Fact<'ctx>),
    Deduce(Fact<'ctx>),
    Assume(Fact<'ctx>),
    _Shorthand(Ustr, FragmentId<'ctx>),
}

impl<'ctx> ProofState<'ctx> {
    fn name_ctx<'a>(&'a self) -> NameCtx<'ctx, 'a> {
        NameCtx::new(&self.templates, &self.shorthands)
    }

    pub fn reasoning_chain(&self) -> &[ReasoningStep<'ctx>] {
        &self.reasoning_chain
    }

    pub fn goal(&self) -> FragmentId<'ctx> {
        self.goal
    }
}

generate_arena_handle!(ProofStateKey<'ctx> => ProofState<'ctx>);

fn check_proof<'ctx>(
    theorem_smt: TheoremId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<ProofStatus<'ctx>> {
    let proof = match theorem_smt.proof() {
        UnresolvedProof::Axiom => return Ok(ProofStatus::new_axiom()),
        UnresolvedProof::Theorem(proof) => *proof,
    };
    let start_state = ctx.arenas.proof_states.alloc(ProofState {
        knowns: theorem_smt.hypotheses().iter().cloned().collect(),
        reasoning_chain: theorem_smt
            .hypotheses()
            .iter()
            .map(|h| ReasoningStep::Hypothesis(*h))
            .collect(),
        goal: theorem_smt.conclusion(),
        templates: theorem_smt
            .templates()
            .iter()
            .map(|t| (t.name(), t.clone()))
            .collect(),
        shorthands: FxHashMap::default(),
    });

    let mut proof_states = vec![(start_state, proof)];
    let mut proof_correct = true;
    let mut todo_used = false;
    let mut theorems_used = FxHashSet::default();

    while let Some((state, proof)) = proof_states.pop() {
        let tactic = elaborate_tactic(proof, ctx)?;

        match tactic {
            UnresolvedTactic::None => {
                let goal_fact = Fact::new(None, state.goal);
                if !state.knowns.contains(&goal_fact) {
                    ctx.diags
                        .err_missing_goal((theorem_smt, state), proof.span());
                    proof_correct = false;
                }
            }
            UnresolvedTactic::Have(tactic) => {
                let mut sub_state = state.0.clone();

                // Add the assumption to the sub-state if there is one.
                let assumption = if let Some(assumption) = tactic.fact.assumption() {
                    let Ok(assumption) = parse_fragment(
                        assumption,
                        ctx.sentence_formal_cat,
                        &mut sub_state.name_ctx(),
                        ctx,
                    ) else {
                        // There isn't much we can do if the assumption doesn't parse
                        // we just drop this state and the continuation too.
                        eprintln!(
                            "[{}] Proof incorrect from parse failure.",
                            theorem_smt.name()
                        );
                        proof_correct = false;
                        continue;
                    };

                    let assumption_fact = Fact::new(None, assumption);
                    sub_state.knowns.insert(assumption_fact);
                    sub_state
                        .reasoning_chain
                        .push(ReasoningStep::Assume(assumption_fact));
                    Some(assumption)
                } else {
                    None
                };

                let Ok(conclusion) = parse_fragment(
                    tactic.fact.conclusion(),
                    ctx.sentence_formal_cat,
                    &mut state.name_ctx(),
                    ctx,
                ) else {
                    eprintln!(
                        "[{}] Proof incorrect from parse failure.",
                        theorem_smt.name()
                    );
                    proof_correct = false;
                    continue;
                };
                sub_state.goal = conclusion;

                let sub_state = ctx.arenas.proof_states.alloc(sub_state);
                proof_states.push((sub_state, tactic.proof));

                let conclusion_fact = Fact::new(assumption, conclusion);

                let mut state = state.0.clone();
                state.knowns.insert(conclusion_fact);
                state
                    .reasoning_chain
                    .push(ReasoningStep::Deduce(conclusion_fact));
                let state = ctx.arenas.proof_states.alloc(state);
                proof_states.push((state, tactic.continuation));
            }
            UnresolvedTactic::By(tactic) => {
                let Some(theorem) = ctx.arenas.theorem_stmts.get(tactic.theorem_name) else {
                    // The theorem doesn't exist.
                    eprintln!(
                        "[{}] Proof incorrect from non-existent theorem.",
                        theorem_smt.name()
                    );
                    proof_correct = false;
                    ctx.diags
                        .err_non_existent_theorem(tactic.theorem_name, tactic.theorem_name_span);
                    continue;
                };

                theorems_used.insert(theorem);

                if theorem.templates().len() != tactic.templates.len() {
                    // The number of templates doesn't match.
                    eprintln!(
                        "[{}] Proof incorrect from template count mismatch.",
                        theorem.name()
                    );
                    proof_correct = false;
                    if theorem.templates().len() > tactic.templates.len() {
                        ctx.diags.err_missing_tactic_templates(
                            tactic
                                .templates
                                .last()
                                .map(|t| t.span())
                                .unwrap_or(tactic.theorem_name_span),
                            theorem.templates().len() - tactic.templates.len(),
                        );
                    } else {
                        ctx.diags.err_extra_tactic_templates(
                            tactic.templates[theorem.templates().len()].span(),
                            tactic.templates.len() - theorem.templates().len(),
                        );
                    }
                    continue;
                }

                let mut template_instantiations = Vec::new();
                for (template, instantiation) in
                    theorem.0.templates().iter().zip(tactic.templates.iter())
                {
                    let mut names = state.name_ctx();

                    for &param_cat in template.params() {
                        names.add_hole(param_cat)
                    }

                    let Ok(instantiation) =
                        parse_any_fragment(*instantiation, template.cat(), &mut names, ctx)
                    else {
                        eprintln!(
                            "[{}] Proof incorrect from parse failure.",
                            theorem_smt.name()
                        );
                        proof_correct = false;
                        continue;
                    };

                    template_instantiations.push(instantiation);
                }

                if template_instantiations.len() != theorem.templates().len() {
                    // One of the template instantiations had an error.
                    continue;
                }

                let mut templates = FxHashMap::default();
                for (i, template) in theorem.templates().iter().enumerate() {
                    templates.insert(template.name(), template_instantiations[i]);
                }

                for &hypothesis in theorem.hypotheses() {
                    let instantiated = instantiate_fact_with_templates(hypothesis, &templates, ctx);

                    if !state.knowns.contains(&instantiated) {
                        dbg!(_debug_fragment(theorem.conclusion()));
                        eprintln!(
                            "[{}] Proof incorrect from missing hypothesis {}.",
                            theorem_smt.name(),
                            _debug_fact(instantiated)
                        );
                        proof_correct = false;
                        // TODO: error message.
                    }
                }

                let conclusion =
                    instantiate_fragment_with_templates(theorem.conclusion(), &templates, ctx);

                if conclusion != state.goal {
                    eprintln!(
                        "[{}] Proof incorrect from theorem {} mismatch goal {}.",
                        theorem_smt.name(),
                        _debug_fragment(conclusion),
                        _debug_fragment(state.goal),
                    );
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

fn instantiate_fact_with_templates<'ctx>(
    fact: Fact<'ctx>,
    templates: &FxHashMap<Ustr, FragmentId<'ctx>>,
    ctx: &Ctx<'ctx>,
) -> Fact<'ctx> {
    let assumption = fact
        .assumption()
        .map(|assump| instantiate_fragment_with_templates(assump, templates, ctx));
    let conclusion = instantiate_fragment_with_templates(fact.conclusion(), templates, ctx);
    Fact::new(assumption, conclusion)
}

fn instantiate_fragment_with_templates<'ctx>(
    frag: FragmentId<'ctx>,
    templates: &FxHashMap<Ustr, FragmentId<'ctx>>,
    ctx: &Ctx<'ctx>,
) -> FragmentId<'ctx> {
    match frag.0.data() {
        FragData::Rule(rule) => {
            let formal_rule = rule.rule();
            let bindings_added = rule.bindings_added();

            let mut new_parts: Vec<FragPart<'ctx>> = Vec::new();
            for part in rule.children() {
                let new_part = match part {
                    FragPart::Fragment(frag_id) => {
                        let new_frag =
                            instantiate_fragment_with_templates(*frag_id, templates, ctx);
                        FragPart::Fragment(new_frag)
                    }
                    FragPart::Variable(cat, idx) => FragPart::Variable(*cat, *idx),
                };
                new_parts.push(new_part);
            }
            let data: FragData<'ctx> = FragData::Rule(FragRuleApplication::new(
                formal_rule,
                new_parts,
                bindings_added,
            ));
            ctx.arenas.fragments.intern(Fragment::new(frag.cat(), data))
        }
        FragData::Template(temp) => {
            let replacement = templates[&temp.name()];
            let args: Vec<_> = temp
                .args()
                .iter()
                .map(|&arg| instantiate_fragment_with_templates(arg, templates, ctx))
                .collect();
            fill_template_holes(replacement, &args, 0, ctx)
        }
        FragData::Hole(_) => unreachable!(),
    }
}

fn fill_template_holes<'ctx>(
    frag: FragmentId<'ctx>,
    args: &[FragmentId<'ctx>],
    debruijn_shift: usize,
    ctx: &Ctx<'ctx>,
) -> FragmentId<'ctx> {
    match frag.0.data() {
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
            ctx.arenas.fragments.intern(Fragment::new(frag.cat(), data))
        }
        FragData::Template(template) => {
            let new_args = template
                .args()
                .iter()
                .map(|arg| fill_template_holes(*arg, args, debruijn_shift, ctx))
                .collect();
            let data = FragData::Template(FragTemplateRef::new(template.name(), new_args));
            ctx.arenas.fragments.intern(Fragment::new(frag.cat(), data))
        }
        FragData::Hole(idx) => {
            let replacement = args[*idx];
            fix_debruijn_indices(replacement, debruijn_shift, ctx)
        }
    }
}

fn fix_debruijn_indices<'ctx>(
    frag: FragmentId<'ctx>,
    shift: usize,
    ctx: &Ctx<'ctx>,
) -> FragmentId<'ctx> {
    if shift == 0 {
        return frag;
    }

    match frag.0.data() {
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
            ctx.arenas.fragments.intern(Fragment::new(frag.cat(), data))
        }
        FragData::Template(template) => {
            let new_args = template
                .args()
                .iter()
                .map(|&arg| fix_debruijn_indices(arg, shift, ctx))
                .collect();
            let data = FragData::Template(FragTemplateRef::new(template.name(), new_args));
            ctx.arenas.fragments.intern(Fragment::new(frag.cat(), data))
        }
        FragData::Hole(_) => frag,
    }
}

#[derive(Debug, Clone)]
pub enum UnresolvedTactic<'ctx> {
    None,
    Have(UnresolvedHaveTactic<'ctx>),
    By(UnresolvedByTactic<'ctx>),
    Todo,
}

#[derive(Debug, Clone, Copy)]
pub struct UnresolvedHaveTactic<'ctx> {
    pub fact: UnresolvedFact<'ctx>,
    pub proof: ParseTreeId<'ctx>,
    pub continuation: ParseTreeId<'ctx>,
}

#[derive(Debug, Clone)]
pub struct UnresolvedByTactic<'ctx> {
    pub theorem_name: Ustr,
    pub theorem_name_span: Span,
    pub templates: Vec<ParseTreeId<'ctx>>,
}
