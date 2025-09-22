use std::ops::Index;

use rustc_hash::FxHashMap;
use slotmap::SecondaryMap;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        elaborator::{elaborate_maybe_shorthand_args, reduce_to_builtin},
        macros::do_macro_replacement,
        parse_state::{ParseRuleSource, SyntaxCategorySource},
        parse_tree::ParseTreeId,
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart},
        fragment::{FragData, FragPart, FragRuleApplication, Fragment, FragmentId},
    },
};

pub struct NameCtx {
    templates: FxHashMap<Ustr, TemplateInfo>,
    shorthands: FxHashMap<Ustr, FragmentId>,
    bindings: SecondaryMap<FormalSyntaxCatId, Vec<Ustr>>,
}

pub struct TemplateInfo {
    cat: FormalSyntaxCatId,
    params: Vec<FormalSyntaxCatId>,
}

pub fn parse_any_fragment(
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

        if let Some(frag_id) = parse_fragment(frag, expected_cat, names, ctx)? {
            possible_formals.push(frag_id);
        }
    }

    match &possible_formals[..] {
        [frag] => Ok(Some(*frag)),
        _ => Ok(None),
    }
}

pub fn parse_fragment(
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
                let name = possibility.children()[0].as_name().unwrap();
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
                    if template.cat != expected_cat {
                        // This template is not for the expected category.
                        continue;
                    }

                    if template.params.len() != args.len() {
                        // Wrong number of arguments.
                        continue;
                    }

                    todo!();
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

                for (child, formal_part) in possibility
                    .children()
                    .iter()
                    .zip(formal_rule.pattern().clone().parts())
                {
                    match formal_part {
                        FormalSyntaxPatPart::Cat(_) => {
                            let Some(child_frag_id) =
                                parse_fragment(child.as_node().unwrap(), expected_cat, names, ctx)?
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

                            let var_name = child.as_name().unwrap();
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
            }
            ParseRuleSource::Macro(macro_id) => {
                // Expand the macro and add the new possibilities to the stack.
                let bindings = &ctx.macros[*macro_id].collect_macro_bindings(&possibility);
                let expanded = do_macro_replacement(tree, bindings, ctx);
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

// // Formal syntax fragments when written in watson source code may be
// // ambiguous between different formal syntax categories. So we don't parse
// // them until we know which category they should be. At the point this
// // function is called, we know the expected category.

// // The first thing we are going to do is expand the given parse tree into
// // one that contains only formal syntax rules, i.e. no macros. This will
// // make it possible to perform name resolution correctly by exposing all
// // bindings.
// let expanded_tree = expand_tree(tree, ctx);

// fn expand_tree(tree: ParseTreeId, ctx: &mut Ctx) -> ParseTreeId {
//     // This function takes a parse tree that may contain macros and expands
//     // all macros into their definitions, producing a parse tree that only
//     // contains formal syntax rules.
//     //
//     // This is necessary because macros can introduce new bindings and
//     // change the structure of the parse tree in ways that affect name
//     // resolution and fragment parsing.
//     //
//     // We are going to expand from the bottom up, so we don't have to expand
//     // the same macro multiple times. So the first step is to expand all the
//     // children of this node, then if this node is a macro we expand it. The
//     // macro might introduce new child macros that need to be expanded so we
//     // repeat until there are no more macros.

//     if !ctx.parse_forest.has_unexpanded_macro(tree) {
//         return tree;
//     }

//     let old_tree = &ctx.parse_forest[tree];
//     let span = old_tree.span();
//     let cat = old_tree.cat();

//     let mut new_possibilities = Vec::new();
//     let mut possibilities_to_expand = old_tree.possibilities().to_vec();

//     while let Some(possibility) = possibilities_to_expand.pop() {
//         // First expand all children.

//         let mut new_children = Vec::with_capacity(possibility.children().len());
//         for child in possibility.children() {
//             match child {
//                 ParseTreePart::Atom(atom) => {
//                     new_children.push(ParseTreePart::Atom(*atom));
//                 }
//                 ParseTreePart::Node { id, span, cat } => {
//                     let expanded = expand_tree(*id, ctx);
//                     new_children.push(ParseTreePart::Node { id: expanded, span: *span, cat: *cat });
//                 }
//             }
//         }

//         let possibility = ParseTreeChildren::new(possibility.rule(), new_children);

//         // Now the children are clean. We either push this possibility or expand
//         // it if it's a macro.

//         let rule = &ctx.parse_state[possibility.rule()];

//         let &ParseRuleSource::Macro(macro_id) = rule.source() else {
//             new_possibilities.push(possibility);
//             continue;
//         };

//         let bindings = &ctx.macros[macro_id].collect_macro_bindings(&possibility);
//         let expanded = do_macro_replacement(tree, bindings, ctx);

//         for possibility in ctx.parse_forest[expanded].possibilities() {
//             possibilities_to_expand.push(possibility.clone());
//         }
//     }

//     let new_tree = ParseTree::new(span, cat, new_possibilities);
//     ctx.parse_forest.get_or_insert(new_tree)
// }
