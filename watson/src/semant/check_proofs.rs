use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{
        Span,
        builtin::{
            TACTICS_BY_RULE, TACTICS_EMPTY_RULE, TACTICS_HAVE_RULE, TACTICS_TODO_RULE,
            elaborate_fact, elaborate_tactic_templates,
        },
        elaborator::reduce_to_builtin,
        macros::Macros,
        parse_tree::ParseTree,
    },
    semant::{
        formal_syntax::FormalSyntax,
        fragments::{Frag, FragCtx, FragData, FragId, FragPart, resolve_fact, resolve_frag},
        theorem::{Fact, Template, TheoremId, TheoremStatements},
        unresolved::{UnresolvedFact, UnresolvedFragment, UnresolvedProof},
    },
    strings,
};
use std::{
    collections::{HashMap, HashSet},
    vec,
};
use ustr::Ustr;

pub fn check_proofs(
    statements: &TheoremStatements,
    proofs: HashMap<TheoremId, UnresolvedProof>,
    formal: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
    ctx: &mut FragCtx,
) -> HashMap<TheoremId, ProofStatus> {
    let mut proof_statuses = HashMap::new();

    for (id, proof) in proofs {
        let status = check_proof(id, proof, statements, formal, macros, diags, ctx);
        proof_statuses.insert(id, status);
    }

    proof_statuses
}

pub struct ProofStatus {
    proof_correct: bool,
    todo_used: bool,
    is_axiom: bool,
    theorems_used: HashSet<TheoremId>,
}

impl ProofStatus {
    pub fn proof_correct(&self) -> bool {
        self.proof_correct
    }

    pub fn todo_used(&self) -> bool {
        self.todo_used
    }

    pub fn theorems_used(&self) -> &HashSet<TheoremId> {
        &self.theorems_used
    }

    pub fn is_axiom(&self) -> bool {
        self.is_axiom
    }
}

fn check_proof(
    id: TheoremId,
    proof: UnresolvedProof,
    statements: &TheoremStatements,
    formal: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
    ctx: &mut FragCtx,
) -> ProofStatus {
    let tactics = match proof {
        UnresolvedProof::Axiom => {
            return ProofStatus {
                proof_correct: true,
                todo_used: false,
                is_axiom: true,
                theorems_used: HashSet::new(),
            };
        }
        UnresolvedProof::Theorem(tactics) => tactics,
    };
    let statement = statements.get(id);
    let templates_map: HashMap<Ustr, &Template> = statement
        .templates()
        .iter()
        .map(|t| (t.name(), t))
        .collect();

    let start_state = ProofState {
        goal: statement.conclusion(),
        knowns: statement.hypotheses().iter().cloned().collect(),
        shorthands: HashMap::new(),
    };
    let mut state_stack = vec![(start_state, tactics)];

    let mut proof_correct = true;
    let mut todo_used = false;
    let mut theorems_used = HashSet::new();

    'state: while let Some((state, tactics)) = state_stack.pop() {
        let Ok(tactics) = partially_elaborate_tactics(tactics, formal, macros, diags) else {
            // Some sort of failure with elaboration.
            todo!("err: failed to elaborate tactics");
        };
        let Some((tactic, rest)) = tactics else {
            // Proof is supposed to be done.
            if state.knowns.contains(&Fact::new(None, state.goal)) {
                // Proof complete!
                continue;
            } else {
                // Proof failed. We never actually proved the goal.
                proof_correct = false;
                continue;
            }
        };

        match tactic {
            PartialTactic::By(by) => {
                if !statements.has(by.theorem) {
                    diags.err_unknown_theorem(id, by.theorem.name(), by.theorem_span);
                    proof_correct = false;
                    continue;
                }

                let theorem_statement = statements.get(by.theorem);
                theorems_used.insert(by.theorem);

                if theorem_statement.templates().len() != by.templates.len() {
                    let diff =
                        theorem_statement.templates().len() as isize - by.templates.len() as isize;
                    if diff > 0 {
                        let last_template = if by.templates.is_empty() {
                            by.theorem_span
                        } else {
                            by.templates[by.templates.len() - 1].span
                        };
                        let last_template = Span::new(last_template.end(), last_template.end());
                        diags.err_missing_tactic_templates(id, last_template, diff as usize);
                    } else {
                        let start = by.templates[theorem_statement.templates().len()].span;
                        let end = by.templates[by.templates.len() - 1].span;
                        let span = start.union(end);
                        diags.err_extra_tactic_templates(id, span, -diff as usize);
                    }
                    proof_correct = false;
                    continue;
                }

                let mut template_replacements = HashMap::new();

                for (template, frag) in theorem_statement.templates().iter().zip(by.templates) {
                    if template.cat() != frag.formal_cat {
                        todo!("err: template formal category mismatch");
                    }

                    let Ok(resolved_frag) = resolve_frag(
                        frag,
                        &templates_map,
                        &state.shorthands,
                        &mut Vec::new(),
                        true,
                        formal,
                        ctx,
                        diags,
                        id,
                        Some(&state),
                    ) else {
                        proof_correct = false;
                        continue 'state;
                    };
                    template_replacements.insert(template.name(), resolved_frag);
                }

                for hypothesis in theorem_statement.hypotheses() {
                    let hypothesis_instantiated =
                        replace_templates_fact(hypothesis, &template_replacements, ctx);
                    if !state.knowns.contains(&hypothesis_instantiated) {
                        todo!("err: missing hypothesis");
                    }
                }

                let conclusion =
                    replace_templates(theorem_statement.conclusion(), &template_replacements, ctx);

                if conclusion != state.goal {
                    todo!("err: theorem conclusion mismatch");
                }

                let mut new_state = state;
                new_state.knowns.insert(Fact::new(None, conclusion));
                state_stack.push((new_state, rest));
            }
            PartialTactic::Have(have) => {
                let Ok(goal) = resolve_fact(
                    *have.goal,
                    &templates_map,
                    &state.shorthands,
                    &mut Vec::new(),
                    false,
                    formal,
                    ctx,
                    diags,
                    id,
                    Some(&state),
                ) else {
                    proof_correct = false;
                    continue;
                };

                let mut with_fact = state.clone();
                with_fact.knowns.insert(goal);
                state_stack.push((with_fact, rest));

                let mut prove_goal = state;
                if let Some(assume) = goal.assumption() {
                    prove_goal.knowns.insert(Fact::new(None, assume));
                }
                prove_goal.goal = goal.sentence();
                state_stack.push((prove_goal, have.proof));
            }
            PartialTactic::Todo => {
                let mut next_state = state;
                next_state.knowns.insert(Fact::new(None, next_state.goal));
                state_stack.push((next_state, rest));

                todo_used = true;
            }
        }
    }

    ProofStatus {
        proof_correct,
        todo_used,
        theorems_used,
        is_axiom: false,
    }
}

fn replace_templates_fact(
    fact: &Fact,
    replacements: &HashMap<Ustr, FragId>,
    ctx: &mut FragCtx,
) -> Fact {
    let new_assumption = fact
        .assumption()
        .map(|assump| replace_templates(assump, replacements, ctx));
    let new_sentence = replace_templates(fact.sentence(), replacements, ctx);
    Fact::new(new_assumption, new_sentence)
}

fn replace_templates(
    sentence: FragId,
    replacements: &HashMap<Ustr, FragId>,
    ctx: &mut FragCtx,
) -> FragId {
    let frag = ctx.get(sentence);
    let cat = frag.cat();

    match frag.data().clone() {
        FragData::Rule {
            rule,
            bindings,
            parts,
        } => {
            let new_parts = parts
                .iter()
                .map(|part| match part {
                    // Justification for why we don't need to update the
                    // deBruijn indices: Nothing was added above this rule so
                    // no new bindings could have been introduced.
                    FragPart::Var(var) => FragPart::Var(*var),
                    FragPart::Frag(frag_id) => {
                        let new_frag_id = replace_templates(*frag_id, replacements, ctx);
                        FragPart::Frag(new_frag_id)
                    }
                })
                .collect();
            let data = FragData::Rule {
                rule,
                bindings,
                parts: new_parts,
            };
            ctx.get_or_insert(Frag::new(cat, data))
        }
        FragData::Template { name, args } => {
            let replacement = *replacements.get(&name).expect("no replacement found");
            let new_args: Vec<FragId> = args
                .iter()
                .map(|arg| replace_templates(*arg, replacements, ctx))
                .collect();
            fill_holes(replacement, &new_args, 0, ctx)
        }
        // The conclusion must not have holes.
        FragData::TemplateArgHole(_) => unreachable!(),
    }
}

fn fill_holes(
    frag_id: FragId,
    template_args: &[FragId],
    debruijn_shift: usize,
    ctx: &mut FragCtx,
) -> FragId {
    let frag = ctx.get(frag_id);
    let cat = frag.cat();

    match frag.data().clone() {
        FragData::Rule {
            rule,
            bindings,
            parts,
        } => {
            let new_parts = parts
                .iter()
                .map(|part| match part {
                    FragPart::Var(var) => FragPart::Var(*var),
                    FragPart::Frag(frag_id) => FragPart::Frag(fill_holes(
                        *frag_id,
                        template_args,
                        debruijn_shift + bindings,
                        ctx,
                    )),
                })
                .collect();
            let data = FragData::Rule {
                rule,
                bindings,
                parts: new_parts,
            };
            ctx.get_or_insert(Frag::new(cat, data))
        }
        FragData::Template { name, args } => {
            let new_args = args
                .iter()
                .map(|arg| fill_holes(*arg, template_args, debruijn_shift, ctx))
                .collect();
            let data = FragData::Template {
                name,
                args: new_args,
            };
            ctx.get_or_insert(Frag::new(cat, data))
        }
        FragData::TemplateArgHole(idx) => {
            let fill = template_args
                .get(idx)
                .cloned()
                .expect("template arg hole out of bounds");
            fix_debruijn_indices(fill, debruijn_shift, ctx)
        }
    }
}

fn fix_debruijn_indices(frag_id: FragId, shift: usize, ctx: &mut FragCtx) -> FragId {
    if shift == 0 {
        return frag_id;
    }

    let frag = ctx.get(frag_id);
    let cat = frag.cat();

    match frag.data().clone() {
        FragData::Rule {
            rule,
            bindings,
            parts,
        } => {
            let new_parts = parts
                .iter()
                .map(|part| match part {
                    FragPart::Var(var) => FragPart::Var(var + shift),
                    FragPart::Frag(frag_id) => {
                        FragPart::Frag(fix_debruijn_indices(*frag_id, shift, ctx))
                    }
                })
                .collect();
            let data = FragData::Rule {
                rule,
                bindings,
                parts: new_parts,
            };
            ctx.get_or_insert(Frag::new(cat, data))
        }
        FragData::Template { name, args } => {
            let new_args = args
                .iter()
                .map(|arg| fix_debruijn_indices(*arg, shift, ctx))
                .collect();
            let data = FragData::Template {
                name,
                args: new_args,
            };
            ctx.get_or_insert(Frag::new(cat, data))
        }
        FragData::TemplateArgHole(_) => frag_id,
    }
}

#[derive(Debug, Clone)]
pub struct ProofState {
    goal: FragId,
    knowns: HashSet<Fact>,
    shorthands: HashMap<Ustr, FragId>,
}

impl ProofState {
    pub fn goal(&self) -> FragId {
        self.goal
    }

    pub fn knowns(&self) -> &HashSet<Fact> {
        &self.knowns
    }

    pub fn shorthands(&self) -> &HashMap<Ustr, FragId> {
        &self.shorthands
    }
}

#[derive(Debug)]
enum PartialTactic {
    By(ByTactic),
    Have(HaveTactic),
    Todo,
}

#[derive(Debug)]
struct ByTactic {
    theorem: TheoremId,
    theorem_span: Span,
    templates: Vec<UnresolvedFragment>,
}

#[derive(Debug)]
struct HaveTactic {
    goal: Box<UnresolvedFact>,
    proof: ParseTree,
}

fn partially_elaborate_tactics(
    tactics: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<Option<(PartialTactic, ParseTree)>> {
    let tactics = reduce_to_builtin(tactics, macros, diags)?;

    // tactic ::= <empty>
    //          |"by" name tactic_templates tactics
    //          | "have" fact tactics ";" tactics
    //          | "todo" tactics

    if let Some([]) = tactics.as_rule(*TACTICS_EMPTY_RULE) {
        Ok(None)
    } else if let Some([by_kw, name, templates, rest]) = tactics.as_rule(*TACTICS_BY_RULE) {
        assert!(by_kw.is_kw(*strings::BY));
        let theorem_id = TheoremId::new(name.as_name().unwrap());
        let partial_tactic = PartialTactic::By(ByTactic {
            theorem: theorem_id,
            theorem_span: name.span(),
            templates: elaborate_tactic_templates(templates.clone(), formal_syntax, macros, diags)?,
        });
        Ok(Some((partial_tactic, rest.clone())))
    } else if let Some([have_kw, fact, tactics, semi, rest]) = tactics.as_rule(*TACTICS_HAVE_RULE) {
        assert!(have_kw.is_kw(*strings::HAVE));
        assert!(semi.is_lit(*strings::SEMICOLON));
        let fact = elaborate_fact(fact.clone(), formal_syntax, macros, diags)?;
        let partial_tactic = PartialTactic::Have(HaveTactic {
            goal: Box::new(fact),
            proof: tactics.clone(),
        });
        Ok(Some((partial_tactic, rest.clone())))
    } else if let Some([todo_kw, rest]) = tactics.as_rule(*TACTICS_TODO_RULE) {
        assert!(todo_kw.is_kw(*strings::TODO));
        Ok(Some((PartialTactic::Todo, rest.clone())))
    } else {
        panic!("Failed to match builtin rule.")
    }
}
