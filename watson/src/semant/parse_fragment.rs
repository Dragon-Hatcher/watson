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

#[derive(Debug, Clone)]
pub struct NameCtx<'ctx> {
    templates: FxHashMap<Ustr, Template<'ctx>>,
    shorthands: FxHashMap<Ustr, FragmentId<'ctx>>,
    bindings: Vec<(FormalSyntaxCatId<'ctx>, Ustr)>,
    holes: Vec<FormalSyntaxCatId<'ctx>>,
}

impl<'ctx> NameCtx<'ctx> {
    pub fn new() -> Self {
        Self {
            templates: FxHashMap::default(),
            shorthands: FxHashMap::default(),
            bindings: Vec::new(),
            holes: Vec::new(),
        }
    }

    pub fn add_template(&mut self, name: Ustr, template: Template<'ctx>) {
        self.templates.insert(name, template);
    }

    pub fn add_hole(&mut self, cat: FormalSyntaxCatId<'ctx>) {
        self.holes.push(cat);
    }

    pub fn clear_holes(&mut self) {
        self.holes.clear();
    }
}

pub fn parse_fact<'ctx>(
    fact: UnresolvedFact<'ctx>,
    names: &mut NameCtx<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Fact<'ctx>> {
    let sentence_cat = ctx.sentence_formal_cat;

    let assumption = if let Some(assumption_tree) = fact.assumption {
        Some(parse_fragment(assumption_tree, sentence_cat, names, ctx)?)
    } else {
        None
    };
    let conclusion = parse_fragment(fact.conclusion, sentence_cat, names, ctx)?;

    Ok(Fact::new(assumption, conclusion))
}

pub fn parse_any_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<FragmentId<'ctx>> {
    let Some(frag) = maybe_parse_any_fragment(tree, expected_cat, names, ctx)? else {
        // TODO: actual error message
        return ctx.diags.err_ambiguous_parse(tree.span());
    };

    Ok(frag)
}

pub fn parse_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<FragmentId<'ctx>> {
    let Some(frag) = maybe_parse_fragment(tree, expected_cat, names, ctx)? else {
        // TODO: actual error message
        return ctx.diags.err_ambiguous_parse(tree.span());
    };

    Ok(frag)
}

fn maybe_parse_any_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Option<FragmentId<'ctx>>> {
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
    names: &mut NameCtx<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Option<FragmentId<'ctx>>> {
    debug_assert!(matches!(
        tree.cat().source(),
        SyntaxCategorySource::FormalLang(_)
    ));

    let mut possibilities_todo = tree.possibilities().to_vec();
    let mut successful_fragments: Vec<FragmentId> = Vec::new();

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
                    successful_fragments.push(frag_id);
                    continue;
                } else if let Some(replacement) = names.shorthands.get(&name) {
                    // This is a shorthand for a fragment. We can use it directly.

                    if !args.is_empty() {
                        // Shorthands cannot take arguments.
                        continue;
                    }

                    if replacement.cat() != expected_cat {
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
                        let frag_id = ctx.arenas.fragments.intern(frag);
                        successful_fragments.push(frag_id);
                    }
                } else {
                    // This is not a valid shorthand or template.
                    continue;
                }
            }
            ParseRuleSource::FormalLang(formal_rule) => {
                let mut frag_parts = Vec::new();
                let mut rule_success = true;

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
                    }
                }

                for (child, formal_part) in possibility
                    .children()
                    .iter()
                    .zip(formal_rule.pattern().parts())
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
                    let frag_data = FragData::Rule(FragRuleApplication::new(
                        *formal_rule,
                        frag_parts,
                        binding_count,
                    ));
                    let frag = Fragment::new(formal_rule.cat(), frag_data);
                    let frag_id = ctx.arenas.fragments.intern(frag);
                    successful_fragments.push(frag_id);
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
