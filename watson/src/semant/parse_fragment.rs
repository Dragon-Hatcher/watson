use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        elaborator::{elaborate_maybe_shorthand_args, elaborate_name, reduce_to_builtin},
        macros::do_macro_replacement,
        parse_state::{ParseRuleSource, SyntaxCategorySource},
        parse_tree::ParseTreeId,
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart},
        fragment::{
            FragData, FragPart, FragRuleApplication, FragTemplateRef, Fragment, FragmentId,
        },
        presentation::{
            FactPresentation, PresPart, PresRuleApplication, PresTemplate, PresTreeChild,
            PresTreeData, PresTreeRuleApp, PresTreeTemplate, Presentation, PresentationTree,
            PresentationTreeId,
        },
        theorems::{Fact, Template},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct UnresolvedFact<'ctx> {
    assumption: Option<ParseTreeId<'ctx>>,
    conclusion: ParseTreeId<'ctx>,
}

impl<'ctx> UnresolvedFact<'ctx> {
    pub fn new(assumption: Option<ParseTreeId<'ctx>>, conclusion: ParseTreeId<'ctx>) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }

    pub fn assumption(&self) -> Option<ParseTreeId<'ctx>> {
        self.assumption
    }

    pub fn conclusion(&self) -> ParseTreeId<'ctx> {
        self.conclusion
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameCtx<'ctx, 'a> {
    templates: &'a FxHashMap<Ustr, Template<'ctx>>,
    shorthands: &'a FxHashMap<Ustr, (FragmentId<'ctx>, PresentationTreeId<'ctx>)>,
    bindings: Vec<(FormalSyntaxCatId<'ctx>, Ustr)>,
    holes: Vec<FormalSyntaxCatId<'ctx>>,
}

impl<'ctx, 'a> NameCtx<'ctx, 'a> {
    pub fn new(
        templates: &'a FxHashMap<Ustr, Template<'ctx>>,
        shorthands: &'a FxHashMap<Ustr, (FragmentId<'ctx>, PresentationTreeId<'ctx>)>,
    ) -> Self {
        Self {
            templates,
            shorthands,
            bindings: Vec::new(),
            holes: Vec::new(),
        }
    }

    pub fn add_hole(&mut self, cat: FormalSyntaxCatId<'ctx>) {
        self.holes.push(cat);
    }
}

pub fn parse_fact<'ctx>(
    fact: UnresolvedFact<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(Fact<'ctx>, FactPresentation<'ctx>)> {
    let sentence_cat = ctx.sentence_formal_cat;

    let (assumption, assumption_pres) = if let Some(assumption_tree) = fact.assumption {
        let (frag, pres) = parse_fragment(assumption_tree, sentence_cat, names, ctx)?;
        (Some(frag), Some(pres))
    } else {
        (None, None)
    };
    let (conclusion, conclusion_pres) = parse_fragment(fact.conclusion, sentence_cat, names, ctx)?;

    Ok((
        Fact::new(assumption, conclusion),
        FactPresentation::new(assumption_pres, conclusion_pres),
    ))
}

pub fn parse_any_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(FragmentId<'ctx>, PresentationTreeId<'ctx>)> {
    let Some(frag) = maybe_parse_any_fragment(tree, expected_cat, names, ctx)? else {
        // TODO: actual error message
        return ctx.diags.err_ambiguous_parse(tree.span());
    };

    Ok(frag)
}

pub fn parse_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(FragmentId<'ctx>, PresentationTreeId<'ctx>)> {
    let Some(frag) = maybe_parse_fragment(tree, expected_cat, names, ctx)? else {
        // TODO: actual error message
        return ctx.diags.err_ambiguous_parse(tree.span());
    };

    Ok(frag)
}

fn maybe_parse_any_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Option<(FragmentId<'ctx>, PresentationTreeId<'ctx>)>> {
    debug_assert!(tree.cat() == ctx.builtin_cats.any_fragment);

    let tree = reduce_to_builtin(tree, ctx)?;
    let mut possible_formals = Vec::new();

    for possibility in tree.possibilities() {
        // We reduced the any_fragment to a builtin which means it consists of
        // a single node of a formal fragment.
        let frag = possibility.children()[0].as_node().unwrap();

        let SyntaxCategorySource::FormalLang(formal_cat) = frag.cat().0.source() else {
            panic!("Expected formal syntax category");
        };

        // This isn't the right formal category.
        if formal_cat != expected_cat {
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

fn maybe_parse_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Option<(FragmentId<'ctx>, PresentationTreeId<'ctx>)>> {
    debug_assert!(matches!(
        tree.cat().source(),
        SyntaxCategorySource::FormalLang(_)
    ));

    let mut possibilities_todo = tree.possibilities().to_vec();
    let mut successful_fragments: Vec<(FragmentId, PresentationTreeId)> = Vec::new();

    while let Some(possibility) = possibilities_todo.pop() {
        match possibility.rule().0.source() {
            ParseRuleSource::Builtin => {
                let name = elaborate_name(possibility.children()[0].as_node().unwrap(), ctx)?;
                let args = elaborate_maybe_shorthand_args(
                    possibility.children()[1].as_node().unwrap(),
                    ctx,
                )?;

                if !names.holes.is_empty()
                    && let Some(hole) = parse_hole_name(&name)
                {
                    // This is a hole. We can use it directly.

                    if !args.is_empty() {
                        // Holes cannot take arguments.
                        continue;
                    }

                    if hole >= names.holes.len() {
                        // This hole index is out of bounds.
                        continue;
                    }

                    if names.holes[hole] != expected_cat {
                        // This hole is not for the expected category.
                        continue;
                    }

                    let frag_data = FragData::Hole(hole);
                    let frag = Fragment::new(expected_cat, frag_data);
                    let frag_id = ctx.arenas.fragments.intern(frag);

                    let pres = Presentation::Hole(hole);
                    let pres = ctx.arenas.presentations.intern(pres);

                    let pres_tree = PresentationTree::new(pres, PresTreeData::Hole);
                    let pres_tree = ctx.arenas.presentation_trees.intern(pres_tree);

                    successful_fragments.push((frag_id, pres_tree));
                    continue;
                } else if let Some((replacement, pres_tree)) = names.shorthands.get(&name) {
                    // This is a shorthand for a fragment. We can use it directly.

                    if !args.is_empty() {
                        // Shorthands cannot take arguments.
                        continue;
                    }

                    if replacement.cat() != expected_cat {
                        // This shorthand is not for the expected category.
                        continue;
                    }

                    successful_fragments.push((*replacement, *pres_tree));
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
                    let mut arg_presentations = Vec::new();
                    let mut template_success = true;

                    for (param_cat, arg_frag_id) in
                        template.params().to_vec().iter().zip(args.iter())
                    {
                        let Some((arg_frag, arg_pres)) =
                            maybe_parse_any_fragment(*arg_frag_id, *param_cat, names, ctx)?
                        else {
                            template_success = false;
                            break;
                        };

                        arg_frags.push(arg_frag);
                        arg_presentations.push(arg_pres)
                    }

                    if template_success {
                        let frag_data = FragData::Template(FragTemplateRef::new(name, arg_frags));
                        let frag = Fragment::new(template_cat, frag_data);
                        let frag_id = ctx.arenas.fragments.intern(frag);

                        let pres = PresTemplate::new(
                            name,
                            arg_presentations.iter().map(|p| p.pres()).collect(),
                        );
                        let pres = Presentation::Template(pres);
                        let pres = ctx.arenas.presentations.intern(pres);

                        let pres_tree =
                            PresTreeData::Template(PresTreeTemplate::new(arg_presentations));
                        let pres_tree = PresentationTree::new(pres, pres_tree);
                        let pres_tree = ctx.arenas.presentation_trees.intern(pres_tree);

                        successful_fragments.push((frag_id, pres_tree));
                    }
                } else {
                    // This is not a valid shorthand or template.
                    continue;
                }
            }
            ParseRuleSource::FormalLang(formal_rule) => {
                let mut frag_parts = Vec::new();
                let mut pres_parts = Vec::new();
                let mut pres_tree_parts = Vec::new();
                let mut rule_success = true;

                let mut binding_names = Vec::new();
                let mut binding_names_idx = 0;

                // First push the bindings from this rule to the name context.
                let mut binding_count = 0;
                for (child, formal_part) in possibility
                    .children()
                    .iter()
                    .zip(formal_rule.pattern().parts())
                {
                    if let FormalSyntaxPatPart::Binding(var_formal_cat) = formal_part {
                        let var_name = elaborate_name(child.as_node().unwrap(), ctx)?;
                        names.bindings.push((*var_formal_cat, var_name));
                        binding_count += 1;

                        binding_names.push(var_name);
                    }
                }

                for (child, formal_part) in possibility
                    .children()
                    .iter()
                    .zip(formal_rule.pattern().parts())
                {
                    match formal_part {
                        FormalSyntaxPatPart::Cat(cat) => {
                            let Some((child_frag_id, pres_tree)) =
                                maybe_parse_fragment(child.as_node().unwrap(), *cat, names, ctx)?
                            else {
                                rule_success = false;
                                break;
                            };
                            frag_parts.push(FragPart::Fragment(child_frag_id));
                            pres_tree_parts.push(PresTreeChild::Fragment(pres_tree));
                            pres_parts.push(PresPart::Subpart(vec![pres_tree_parts.len() - 1]));
                        }
                        FormalSyntaxPatPart::Var(var_formal_cat) => {
                            // We need to check the names environment for a binding with this name.

                            let var_name = elaborate_name(child.as_node().unwrap(), ctx)?;
                            let Some((idx, (cat, _))) = names
                                .bindings
                                .iter()
                                .enumerate()
                                .rev()
                                .find(|(_, b)| b.1 == var_name)
                            else {
                                rule_success = false;
                                break;
                            };

                            if cat != var_formal_cat {
                                // This variable was bound with a different category.
                                rule_success = false;
                                break;
                            }

                            frag_parts.push(FragPart::Variable(*var_formal_cat, idx));
                            pres_tree_parts.push(PresTreeChild::Variable);
                            pres_parts.push(PresPart::Variable(var_name));
                        }
                        FormalSyntaxPatPart::Lit(lit) => {
                            pres_parts.push(PresPart::Str(*lit));
                        }
                        FormalSyntaxPatPart::Binding(_cat) => {
                            pres_parts.push(PresPart::Binding(binding_names[binding_names_idx]));
                            binding_names_idx += 1;
                        }
                    }
                }

                if rule_success {
                    // This possibility was successful. We can construct a fragment for it.
                    let frag_data = FragData::Rule(FragRuleApplication::new(
                        *formal_rule,
                        frag_parts,
                        binding_count,
                    ));
                    let frag = Fragment::new(formal_rule.cat(), frag_data);
                    let frag_id = ctx.arenas.fragments.intern(frag);

                    let pres = Presentation::Rule(PresRuleApplication::new(pres_parts));
                    let pres = ctx.arenas.presentations.intern(pres);

                    let pres_tree = PresTreeRuleApp::new(pres_tree_parts);
                    let pres_tree = PresentationTree::new(pres, PresTreeData::Rule(pres_tree));
                    let pres_tree = ctx.arenas.presentation_trees.intern(pres_tree);

                    successful_fragments.push((frag_id, pres_tree));
                }

                // Now pop the bindings we added to the name context.
                names
                    .bindings
                    .truncate(names.bindings.len() - binding_count);
            }
            ParseRuleSource::Macro(mac) => {
                // Expand the macro and add the new possibilities to the stack.
                let bindings = mac.collect_macro_bindings(&possibility);
                let expanded = do_macro_replacement(mac.replacement(), &bindings, ctx);

                for possibility in expanded.possibilities() {
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

fn parse_hole_name(name: &str) -> Option<usize> {
    if name == "_" {
        Some(0)
    } else if let Some(idx) = name.strip_prefix('_') {
        idx.parse().ok()
    } else {
        None
    }
}
