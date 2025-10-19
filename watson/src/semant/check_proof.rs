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
            _debug_fact, FragData, FragPart, FragRuleApplication, FragTemplateRef, Fragment,
            FragmentId,
        },
        parse_fragment::{NameCtx, UnresolvedFact, parse_any_fragment, parse_fragment},
        presentation::{
            FactPresentation, PresTemplate, PresTreeChild, PresTreeData, PresTreeRuleApp,
            PresTreeTemplate, Presentation, PresentationId, PresentationTree, PresentationTreeId,
        },
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
    knowns: FxHashMap<Fact<'ctx>, FactPresentation<'ctx>>,
    goal: (FragmentId<'ctx>, PresentationTreeId<'ctx>),
    templates: FxHashMap<Ustr, Template<'ctx>>,
    shorthands: FxHashMap<Ustr, (FragmentId<'ctx>, PresentationTreeId<'ctx>)>,

    reasoning_chain: Vec<ReasoningStep<'ctx>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningStep<'ctx> {
    Hypothesis((Fact<'ctx>, FactPresentation<'ctx>)),
    Deduce((Fact<'ctx>, FactPresentation<'ctx>)),
    Assume((Fact<'ctx>, FactPresentation<'ctx>)),
    _Shorthand(Ustr, (FragmentId<'ctx>, PresentationTreeId<'ctx>)),
}

impl<'ctx> ProofState<'ctx> {
    fn name_ctx<'a>(&'a self) -> NameCtx<'ctx, 'a> {
        NameCtx::new(&self.templates, &self.shorthands)
    }

    pub fn reasoning_chain(&self) -> &[ReasoningStep<'ctx>] {
        &self.reasoning_chain
    }

    pub fn goal(&self) -> (FragmentId<'ctx>, PresentationTreeId<'ctx>) {
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
                let goal_fact = Fact::new(None, state.goal.0);
                if !state.knowns.contains_key(&goal_fact) {
                    ctx.diags
                        .err_missing_goal((theorem_smt, state), proof.span());
                    proof_correct = false;
                }
            }
            UnresolvedTactic::Have(tactic) => {
                let mut sub_state = state.0.clone();

                // Add the assumption to the sub-state if there is one.
                let assumption = if let Some(assumption) = tactic.fact.assumption() {
                    let Ok((assumption, assumption_pres)) = parse_fragment(
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
                    let assumption_fact_pres = FactPresentation::new(None, assumption_pres);
                    sub_state
                        .knowns
                        .insert(assumption_fact, assumption_fact_pres);
                    sub_state.reasoning_chain.push(ReasoningStep::Assume((
                        assumption_fact,
                        assumption_fact_pres,
                    )));
                    Some((assumption, assumption_pres))
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

                let conclusion_fact = Fact::new(assumption.map(|a| a.0), conclusion.0);
                let conclusion_fact_pres =
                    FactPresentation::new(assumption.map(|a| a.1), conclusion.1);

                let mut state = state.0.clone();
                state.knowns.insert(conclusion_fact, conclusion_fact_pres);
                state.reasoning_chain.push(ReasoningStep::Deduce((
                    conclusion_fact,
                    conclusion_fact_pres,
                )));
                let state = ctx.arenas.proof_states.alloc(state);
                proof_states.push((state, tactic.continuation));
                proof_states.push((sub_state, tactic.proof));
            }
            UnresolvedTactic::By(tactic) => {
                let Some(theorem) = ctx.arenas.theorem_stmts.get(tactic.theorem_name) else {
                    // The theorem doesn't exist.
                    proof_correct = false;
                    ctx.diags
                        .err_non_existent_theorem(tactic.theorem_name, tactic.theorem_name_span);
                    continue;
                };

                theorems_used.insert(theorem);

                if theorem.templates().len() != tactic.templates.len() {
                    // The number of templates doesn't match.
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
                    let (instantiated, instantiated_pres) =
                        instantiate_fact_with_templates(hypothesis, &templates, ctx);

                    if !state.knowns.contains_key(&instantiated) {
                        ctx.diags.err_missing_hypothesis((theorem_smt, state), proof.span(), instantiated_pres);
                        proof_correct = false;
                    }
                }

                let conclusion =
                    instantiate_fragment_with_templates(theorem.conclusion(), &templates, ctx);

                if conclusion != state.goal {
                    ctx.diags.err_goal_conclusion_mismatch(
                        (theorem_smt, state),
                        proof.span(),
                        conclusion.1,
                    );
                    proof_correct = false;
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
    fact: (Fact<'ctx>, FactPresentation<'ctx>),
    templates: &FxHashMap<Ustr, (FragmentId<'ctx>, PresentationTreeId<'ctx>)>,
    ctx: &Ctx<'ctx>,
) -> (Fact<'ctx>, FactPresentation<'ctx>) {
    let (assumption, assumption_pres) = if let (Some(assumption), Some(assumption_pres)) =
        (fact.0.assumption(), fact.1.assumption())
    {
        let (assumption, assumption_pres) =
            instantiate_fragment_with_templates((assumption, assumption_pres), templates, ctx);
        (Some(assumption), Some(assumption_pres))
    } else {
        (None, None)
    };

    let (conclusion, conclusion_pres) = instantiate_fragment_with_templates(
        (fact.0.conclusion(), fact.1.conclusion()),
        templates,
        ctx,
    );
    (
        Fact::new(assumption, conclusion),
        FactPresentation::new(assumption_pres, conclusion_pres),
    )
}

fn instantiate_fragment_with_templates<'ctx>(
    frag: (FragmentId<'ctx>, PresentationTreeId<'ctx>),
    templates: &FxHashMap<Ustr, (FragmentId<'ctx>, PresentationTreeId<'ctx>)>,
    ctx: &Ctx<'ctx>,
) -> (FragmentId<'ctx>, PresentationTreeId<'ctx>) {
    match (frag.0.0.data(), frag.1.data(), frag.1.pres().0) {
        (FragData::Rule(rule), PresTreeData::Rule(pres_tree), Presentation::Rule(_)) => {
            let formal_rule = rule.rule();
            let bindings_added = rule.bindings_added();

            let mut new_children = Vec::new();
            let mut new_children_pres = Vec::new();
            for (part, part_pres) in rule.children().iter().zip(pres_tree.children().iter()) {
                let (new_part, new_pres) = match (part, part_pres) {
                    (FragPart::Fragment(frag_id), PresTreeChild::Fragment(frag_pres)) => {
                        let (new_frag, new_pres) = instantiate_fragment_with_templates(
                            (*frag_id, *frag_pres),
                            templates,
                            ctx,
                        );
                        (
                            FragPart::Fragment(new_frag),
                            PresTreeChild::Fragment(new_pres),
                        )
                    }
                    (FragPart::Variable(cat, idx), PresTreeChild::Variable) => {
                        (FragPart::Variable(*cat, *idx), PresTreeChild::Variable)
                    }
                    _ => unreachable!(),
                };
                new_children.push(new_part);
                new_children_pres.push(new_pres);
            }
            let data: FragData<'ctx> = FragData::Rule(FragRuleApplication::new(
                formal_rule,
                new_children,
                bindings_added,
            ));
            let new_frag = ctx
                .arenas
                .fragments
                .intern(Fragment::new(frag.0.cat(), data));

            let new_pres_tree = new_rule_pres_tree(frag.1.pres(), new_children_pres, ctx);

            (new_frag, new_pres_tree)
        }
        (FragData::Template(temp), PresTreeData::Template(pres), Presentation::Template(_)) => {
            let replacement = templates[&temp.name()];
            let args: Vec<_> = temp
                .args()
                .iter()
                .zip(pres.args().iter())
                .map(|(arg, pres)| {
                    instantiate_fragment_with_templates((*arg, *pres), templates, ctx)
                })
                .collect();
            fill_template_holes(replacement, &args, 0, ctx)
        }
        _ => unreachable!(),
    }
}

fn fill_template_holes<'ctx>(
    frag: (FragmentId<'ctx>, PresentationTreeId<'ctx>),
    args: &[(FragmentId<'ctx>, PresentationTreeId<'ctx>)],
    debruijn_shift: usize,
    ctx: &Ctx<'ctx>,
) -> (FragmentId<'ctx>, PresentationTreeId<'ctx>) {
    match (frag.0.0.data(), frag.1.data(), frag.1.pres().0) {
        (FragData::Rule(rule_app), PresTreeData::Rule(pres_tree), Presentation::Rule(_)) => {
            let (new_children, new_children_pres): (Vec<_>, Vec<_>) = rule_app
                .children()
                .iter()
                .zip(pres_tree.children().iter())
                .map(|(child, child_pres)| match (child, child_pres) {
                    (FragPart::Fragment(child_id), PresTreeChild::Fragment(pres_id)) => {
                        let (new_child, new_pres) = fill_template_holes(
                            (*child_id, *pres_id),
                            args,
                            debruijn_shift + rule_app.bindings_added(),
                            ctx,
                        );
                        (
                            FragPart::Fragment(new_child),
                            PresTreeChild::Fragment(new_pres),
                        )
                    }
                    (FragPart::Variable(_, _), PresTreeChild::Variable) => (*child, *child_pres),
                    _ => unreachable!(),
                })
                .unzip();

            let data = FragData::Rule(FragRuleApplication::new(
                rule_app.rule(),
                new_children,
                rule_app.bindings_added(),
            ));
            let new_frag = ctx
                .arenas
                .fragments
                .intern(Fragment::new(frag.0.cat(), data));

            let new_pres_tree = new_rule_pres_tree(frag.1.pres(), new_children_pres, ctx);

            (new_frag, new_pres_tree)
        }
        (
            FragData::Template(template),
            PresTreeData::Template(pres_tree),
            Presentation::Template(pres),
        ) => {
            let (new_args, new_pres_args): (Vec<_>, Vec<_>) = template
                .args()
                .iter()
                .zip(pres_tree.args().iter())
                .map(|(arg, pres)| fill_template_holes((*arg, *pres), args, debruijn_shift, ctx))
                .unzip();
            let data = FragData::Template(FragTemplateRef::new(template.name(), new_args));
            let new_frag = ctx
                .arenas
                .fragments
                .intern(Fragment::new(frag.0.cat(), data));

            let new_pres = PresTemplate::new(
                pres.name(),
                new_pres_args.iter().map(|a| a.pres()).collect(),
            );
            let new_pres = Presentation::Template(new_pres);
            let new_pres = ctx.arenas.presentations.intern(new_pres);

            let new_pres_tree = PresTreeData::Template(PresTreeTemplate::new(new_pres_args));
            let new_pres_tree = PresentationTree::new(new_pres, new_pres_tree);
            let new_pres_tree = ctx.arenas.presentation_trees.intern(new_pres_tree);

            (new_frag, new_pres_tree)
        }
        (FragData::Hole(idx), PresTreeData::Hole, Presentation::Hole(_)) => {
            let (replacement, replacement_pres) = args[*idx];
            let replacement = fix_debruijn_indices(replacement, debruijn_shift, ctx);
            (replacement, replacement_pres)
        }
        _ => unreachable!(),
    }
}

fn new_rule_pres_tree<'ctx>(
    old_pres: PresentationId<'ctx>,
    new_children_pres: Vec<PresTreeChild<'ctx>>,
    ctx: &Ctx<'ctx>,
) -> PresentationTreeId<'ctx> {
    let new_pres_tree = PresTreeData::Rule(PresTreeRuleApp::new(new_children_pres));
    let new_pres_tree = PresentationTree::new(old_pres, new_pres_tree);
    ctx.arenas.presentation_trees.intern(new_pres_tree)
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
