use rustc_hash::FxHashMap;
use slotmap::SecondaryMap;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        elaborator::{elaborate_maybe_shorthand_args, elaborate_name, reduce_to_builtin},
        macros::do_macro_replacement,
        parse_state::{ParseRuleSource, SyntaxCategorySource},
        parse_tree::{_debug_parse_tree, ParseTreeId},
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart},
        fragment::{
            _debug_fragment, FragData, FragPart, FragRuleApplication, FragTemplateRef, Fragment,
            FragmentId,
        },
        theorems::{Fact, Template},
    },
};

pub struct UnresolvedFact {
    assumption: Option<ParseTreeId>,
    conclusion: ParseTreeId,
}

impl UnresolvedFact {
    pub fn new(assumption: Option<ParseTreeId>, conclusion: ParseTreeId) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NameCtx {
    templates: FxHashMap<Ustr, Template>,
    shorthands: FxHashMap<Ustr, FragmentId>,
    bindings: SecondaryMap<FormalSyntaxCatId, Vec<Ustr>>,
}

impl NameCtx {
    pub fn new() -> Self {
        Self {
            templates: FxHashMap::default(),
            shorthands: FxHashMap::default(),
            bindings: SecondaryMap::default(),
        }
    }

    pub fn add_template(&mut self, name: Ustr, template: Template) {
        self.templates.insert(name, template);
    }
}

pub fn parse_fact(fact: UnresolvedFact, names: &mut NameCtx, ctx: &mut Ctx) -> WResult<Fact> {
    let sentence_cat = ctx.formal_syntax.sentence_cat();

    let assumption = if let Some(assumption_tree) = fact.assumption {
        Some(parse_fragment(assumption_tree, sentence_cat, names, ctx)?)
    } else {
        None
    };
    let conclusion = parse_fragment(fact.conclusion, sentence_cat, names, ctx)?;

    Ok(Fact::new(assumption, conclusion))
}

pub fn parse_any_fragment(
    tree: ParseTreeId,
    expected_cat: FormalSyntaxCatId,
    names: &mut NameCtx,
    ctx: &mut Ctx,
) -> WResult<FragmentId> {
    let Some(frag) = maybe_parse_any_fragment(tree, expected_cat, names, ctx)? else {
        // TODO: actual error message
        return ctx.diags.err_ambiguous_parse(ctx.parse_forest[tree].span());
    };

    Ok(frag)
}

pub fn parse_fragment(
    tree: ParseTreeId,
    expected_cat: FormalSyntaxCatId,
    names: &mut NameCtx,
    ctx: &mut Ctx,
) -> WResult<FragmentId> {
    let Some(frag) = maybe_parse_fragment(tree, expected_cat, names, ctx)? else {
        // TODO: actual error message
        return ctx.diags.err_ambiguous_parse(ctx.parse_forest[tree].span());
    };

    Ok(frag)
}

fn maybe_parse_any_fragment(
    tree: ParseTreeId,
    expected_cat: FormalSyntaxCatId,
    names: &mut NameCtx,
    ctx: &mut Ctx,
) -> WResult<Option<FragmentId>> {
    debug_assert!(ctx.parse_forest[tree].cat() == ctx.builtin_cats.any_fragment);

    let tree = reduce_to_builtin(tree, ctx)?;
    let mut possible_formals = Vec::new();

    for possibility in ctx.parse_forest[tree].possibilities().to_owned() {
        // We reduced the any_fragment to a builtin which means it consists of
        // a single node of a formal fragment.
        let frag = possibility.children()[0].as_node().unwrap();
        let cat = ctx.parse_forest[frag].cat();
        let SyntaxCategorySource::FormalLang(formal_cat) = ctx.parse_state[cat].source() else {
            panic!("Expected formal syntax category");
        };

        // This isn't the right formal category.
        if *formal_cat != expected_cat {
            continue;
        }

        if let Some(frag_id) = maybe_parse_fragment(frag, expected_cat, names, ctx)? {
            possible_formals.push(frag_id);
        }
    }

    match &possible_formals[..] {
        [frag] => Ok(Some(*frag)),
        _ => Ok(None),
    }
}

fn maybe_parse_fragment(
    tree: ParseTreeId,
    expected_cat: FormalSyntaxCatId,
    names: &mut NameCtx,
    ctx: &mut Ctx,
) -> WResult<Option<FragmentId>> {
    debug_assert!(matches!(
        ctx.parse_state[ctx.parse_forest[tree].cat()].source(),
        SyntaxCategorySource::FormalLang(_)
    ));

    let mut possibilities_todo = ctx.parse_forest[tree].possibilities().to_vec();
    let mut successful_fragments: Vec<FragmentId> = Vec::new();

    while let Some(possibility) = possibilities_todo.pop() {
        let rule = &ctx.parse_state[possibility.rule()];

        match rule.source() {
            ParseRuleSource::Builtin => {
                let name = elaborate_name(possibility.children()[0].as_node().unwrap(), ctx)?;
                let args = elaborate_maybe_shorthand_args(
                    possibility.children()[1].as_node().unwrap(),
                    ctx,
                )?;

                if let Some(replacement) = names.shorthands.get(&name) {
                    // This is a shorthand for a fragment. We can use it directly.

                    if !args.is_empty() {
                        // Shorthands cannot take arguments.
                        continue;
                    }

                    if ctx.fragments[*replacement].cat() != expected_cat {
                        // This shorthand is not for the expected category.
                        continue;
                    }

                    successful_fragments.push(*replacement);
                    continue;
                } else if let Some(template) = names.templates.get(&name) {
                    let template_cat = template.cat();

                    if template_cat != expected_cat {
                        // This template is not for the expected category.
                        continue;
                    }

                    if template.params().len() != args.len() {
                        // Wrong number of arguments.
                        continue;
                    }

                    let mut arg_frags = Vec::new();
                    let mut template_success = true;

                    for (param_cat, arg_frag_id) in
                        template.params().to_vec().iter().zip(args.iter())
                    {
                        let Some(arg_frag) =
                            maybe_parse_any_fragment(*arg_frag_id, *param_cat, names, ctx)?
                        else {
                            template_success = false;
                            break;
                        };

                        arg_frags.push(arg_frag);
                    }

                    if template_success {
                        let frag_data = FragData::Template(FragTemplateRef::new(name, arg_frags));
                        let frag = Fragment::new(template_cat, frag_data);
                        let frag_id = ctx.fragments.get_or_insert(frag);
                        successful_fragments.push(frag_id);
                    }
                } else {
                    // This is not a valid shorthand or template.
                    continue;
                }
            }
            ParseRuleSource::FormalLang(formal_rule_id) => {
                let formal_rule_id = *formal_rule_id;
                let formal_rule = &ctx.formal_syntax[formal_rule_id];
                let mut frag_parts = Vec::new();
                let mut rule_success = true;

                // First push the bindings from this rule to the name context.
                let mut binding_count: SecondaryMap<FormalSyntaxCatId, usize> =
                    SecondaryMap::default();
                for (child, formal_part) in possibility
                    .children()
                    .iter()
                    .zip(formal_rule.pattern().clone().parts())
                {
                    if let FormalSyntaxPatPart::Binding(var_formal_cat) = formal_part {
                        let var_name = elaborate_name(child.as_node().unwrap(), ctx)?;
                        names
                            .bindings
                            .entry(*var_formal_cat)
                            .unwrap()
                            .or_default()
                            .push(var_name);
                        *binding_count.entry(*var_formal_cat).unwrap().or_default() += 1;
                    }
                }

                let formal_rule = &ctx.formal_syntax[formal_rule_id];

                for (child, formal_part) in possibility
                    .children()
                    .iter()
                    .zip(formal_rule.pattern().clone().parts())
                {
                    match formal_part {
                        FormalSyntaxPatPart::Cat(cat) => {
                            let Some(child_frag_id) =
                                maybe_parse_fragment(child.as_node().unwrap(), *cat, names, ctx)?
                            else {
                                rule_success = false;
                                break;
                            };
                            frag_parts.push(FragPart::Fragment(child_frag_id));
                        }
                        FormalSyntaxPatPart::Var(var_formal_cat) => {
                            // We need to check the names environment for a binding with this name.
                            let Some(bindings) = names.bindings.get(*var_formal_cat) else {
                                rule_success = false;
                                break;
                            };

                            let var_name = elaborate_name(child.as_node().unwrap(), ctx)?;
                            let Some((idx, _)) = bindings
                                .iter()
                                .enumerate()
                                .rev()
                                .find(|(_, b)| **b == var_name)
                            else {
                                rule_success = false;
                                break;
                            };

                            frag_parts.push(FragPart::Variable(*var_formal_cat, idx))
                        }
                        FormalSyntaxPatPart::Lit(_) | FormalSyntaxPatPart::Binding(_) => {
                            // The parser already ensures that these parts of the pattern
                            // are matched so there is nothing extra to do here.
                            continue;
                        }
                    }
                }

                if rule_success {
                    // This possibility was successful. We can construct a fragment for it.
                    let frag_data =
                        FragData::Rule(FragRuleApplication::new(formal_rule_id, frag_parts));
                    let frag = Fragment::new(ctx.formal_syntax[formal_rule_id].cat(), frag_data);
                    let frag_id = ctx.fragments.get_or_insert(frag);
                    successful_fragments.push(frag_id);
                }

                // Now pop the bindings we added to the name context.
                for (cat, count) in binding_count {
                    let bindings = names.bindings.get_mut(cat).unwrap();
                    let new_len = bindings.len() - count;
                    bindings.truncate(new_len);
                }
            }
            ParseRuleSource::Macro(macro_id) => {
                // Expand the macro and add the new possibilities to the stack.
                let bindings = &ctx.macros[*macro_id].collect_macro_bindings(&possibility);
                let expanded =
                    do_macro_replacement(ctx.macros[*macro_id].replacement(), bindings, ctx);

                for possibility in ctx.parse_forest[expanded].possibilities() {
                    possibilities_todo.push(possibility.clone());
                }
            }
        }
    }

    match &successful_fragments[..] {
        [frag] => Ok(Some(*frag)),
        _ => Ok(None),
    }
}
