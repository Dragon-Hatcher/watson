use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        elaborator::{elaborate_maybe_shorthand_args, elaborate_name},
        parse_state::{ParseRuleSource, SyntaxCategorySource},
        parse_tree::{ParseTreeChildren, ParseTreeId},
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart},
        fragment::{
            FragData, FragPart, FragRuleApplication, FragTemplateRef, Fragment, FragmentId,
        },
        presentation::{
            PresPart, PresRuleApplication, PresTemplate, PresTreeChild, PresTreeData,
            PresTreeRuleApp, PresTreeTemplate, Presentation, PresentationTree, PresentationTreeId,
        },
        theorems::Template,
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

pub fn parse_any_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(FragmentId<'ctx>, PresentationTreeId<'ctx>)> {
    let Some((frag, pres, _)) =
        maybe_parse_any_fragment(tree, expected_cat, names, ctx, &mut Vec::new())?
    else {
        return Err(());
    };

    Ok((frag, pres))
}

pub fn parse_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(FragmentId<'ctx>, PresentationTreeId<'ctx>)> {
    let Some((frag, pres, _)) =
        maybe_parse_fragment(tree, expected_cat, names, ctx, &mut Vec::new())?
    else {
        return Err(());
    };

    Ok((frag, pres))
}

fn maybe_parse_any_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
    cur_path: &mut Vec<usize>,
) -> WResult<Option<(FragmentId<'ctx>, PresentationTreeId<'ctx>, Mappings<'ctx>)>> {
    debug_assert!(tree.cat() == ctx.builtin_cats.any_fragment);

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

        if let Some(frag_id) = maybe_parse_fragment(frag, expected_cat, names, ctx, cur_path)? {
            possible_formals.push(frag_id);
        }
    }

    if possible_formals.len() == 1 {
        Ok(Some(possible_formals.pop().unwrap()))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone, Default)]
struct Mappings<'ctx> {
    frags: FxHashMap<ParseTreeId<'ctx>, Vec<usize>>,
    bindings: FxHashMap<ParseTreeId<'ctx>, Ustr>,
    vars: FxHashMap<ParseTreeId<'ctx>, Ustr>,
}

impl<'ctx> Mappings<'ctx> {
    fn new() -> Self {
        Default::default()
    }

    fn merge(&mut self, other: &Self) {
        for (key, path) in &other.frags {
            self.frags.insert(*key, path.clone());
        }
        for (key, bind) in &other.bindings {
            self.bindings.insert(*key, *bind);
        }
        for (key, var) in &other.vars {
            self.vars.insert(*key, *var);
        }
    }
}

#[derive(Debug, Clone)]
struct PartialPresentation<'ctx> {
    pres: Presentation<'ctx>,
    parse_nodes: Vec<ParseTreeId<'ctx>>,
}

impl<'ctx> PartialPresentation<'ctx> {
    fn fix_path(base_path: &[usize], path: &[usize]) -> Vec<usize> {
        path[base_path.len()..].to_vec()
    }

    fn complete(
        mut self,
        base_path: &[usize],
        map: &Mappings<'ctx>,
        alt: &Presentation<'ctx>,
    ) -> Presentation<'ctx> {
        fn update<'ctx>(
            pres: &mut Presentation<'ctx>,
            base_path: &[usize],
            map: &Mappings<'ctx>,
            alt: &Presentation<'ctx>,
            parse_nodes: &[ParseTreeId<'ctx>],
            idx: &mut usize,
        ) {
            if let Presentation::Rule(rule) = pres {
                for part in &mut rule.parts {
                    match part {
                        PresPart::Subpart(v) => {
                            let tree = parse_nodes[*idx];
                            if !v.is_empty() {
                                *part = PresPart::Chain(Box::new(alt.clone()));
                            } else if let Some(path) = map.frags.get(&tree) {
                                *part = PresPart::Subpart(PartialPresentation::fix_path(
                                    base_path, path,
                                ));
                            } else if let Some(bind) = map.bindings.get(&tree) {
                                *part = PresPart::Binding(*bind)
                            } else if let Some(var) = map.vars.get(&tree) {
                                *part = PresPart::Variable(*var)
                            } else {
                                *part = PresPart::Str(Ustr::from("?"))
                            }
                            *idx += 1;
                        }
                        PresPart::Chain(chain) => {
                            update(chain, base_path, map, alt, parse_nodes, idx);
                        }
                        _ => {}
                    }
                }
            }
        }

        update(
            &mut self.pres,
            base_path,
            map,
            alt,
            &self.parse_nodes,
            &mut 0,
        );
        self.pres
    }
}

fn maybe_parse_fragment<'ctx>(
    tree: ParseTreeId<'ctx>,
    expected_cat: FormalSyntaxCatId<'ctx>,
    names: &mut NameCtx<'ctx, '_>,
    ctx: &mut Ctx<'ctx>,
    cur_path: &mut Vec<usize>,
) -> WResult<Option<(FragmentId<'ctx>, PresentationTreeId<'ctx>, Mappings<'ctx>)>> {
    debug_assert!(matches!(
        tree.cat().source(),
        SyntaxCategorySource::FormalLang(_)
    ));

    let mut possibilities_todo: Vec<(ParseTreeChildren<'ctx>, Option<PartialPresentation<'ctx>>)> =
        tree.possibilities()
            .iter()
            .map(|t| (t.clone(), None))
            .collect();
    let mut successful_fragments: Vec<(
        FragmentId<'ctx>,
        PresentationTreeId<'ctx>,
        Mappings<'ctx>,
    )> = Vec::new();

    while let Some((possibility, macro_pres)) = possibilities_todo.pop() {
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

                    successful_fragments.push((frag_id, pres_tree, Mappings::new()));
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

                    successful_fragments.push((*replacement, *pres_tree, Mappings::new()));
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
                    let mut mapping = Mappings::new();

                    for (i, (param_cat, arg_frag_id)) in template
                        .params()
                        .to_vec()
                        .iter()
                        .zip(args.iter())
                        .enumerate()
                    {
                        cur_path.push(i);
                        let parse = maybe_parse_any_fragment(
                            *arg_frag_id,
                            *param_cat,
                            names,
                            ctx,
                            cur_path,
                        );
                        cur_path.pop();
                        let Some((arg_frag, arg_pres, new_mapping)) = parse? else {
                            template_success = false;
                            break;
                        };
                        mapping.merge(&new_mapping);

                        arg_frags.push(arg_frag);
                        arg_presentations.push(arg_pres)
                    }

                    if template_success {
                        let frag_data = FragData::Template(FragTemplateRef::new(name, arg_frags));
                        let frag = Fragment::new(template_cat, frag_data);
                        let frag_id = ctx.arenas.fragments.intern(frag);

                        let new_pres = PresTemplate::new(
                            name,
                            arg_presentations.iter().map(|p| p.pres()).collect(),
                        );
                        let new_pres = Presentation::Template(new_pres);

                        let pres = match macro_pres {
                            Some(pres) => pres.complete(cur_path, &mapping, &new_pres),
                            None => new_pres,
                        };
                        let pres = ctx.arenas.presentations.intern(pres);

                        let pres_tree =
                            PresTreeData::Template(PresTreeTemplate::new(arg_presentations));
                        let pres_tree = PresentationTree::new(pres, pres_tree);
                        let pres_tree = ctx.arenas.presentation_trees.intern(pres_tree);

                        successful_fragments.push((frag_id, pres_tree, mapping));
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

                let mut mapping = Mappings::new();

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
                            cur_path.push(pres_tree_parts.len());
                            let parse = maybe_parse_fragment(
                                child.as_node().unwrap(),
                                *cat,
                                names,
                                ctx,
                                cur_path,
                            );
                            cur_path.pop();
                            let Some((child_frag_id, pres_tree, new_mappings)) = parse? else {
                                rule_success = false;
                                break;
                            };
                            mapping.merge(&new_mappings);
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

                            mapping.vars.insert(child.as_node().unwrap(), var_name);
                            frag_parts.push(FragPart::Variable(*var_formal_cat, idx));
                            pres_tree_parts.push(PresTreeChild::Variable);
                            pres_parts.push(PresPart::Variable(var_name));
                        }
                        FormalSyntaxPatPart::Lit(lit) => {
                            pres_parts.push(PresPart::Str(*lit));
                        }
                        FormalSyntaxPatPart::Binding(_cat) => {
                            let var_name = binding_names[binding_names_idx];
                            mapping.vars.insert(child.as_node().unwrap(), var_name);
                            pres_parts.push(PresPart::Binding(var_name));
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

                    let new_pres = Presentation::Rule(PresRuleApplication::new(pres_parts));
                    let pres = match macro_pres {
                        Some(pres) => pres.complete(cur_path, &mapping, &new_pres),
                        None => new_pres,
                    };
                    let pres = ctx.arenas.presentations.intern(pres);

                    let pres_tree = PresTreeRuleApp::new(pres_tree_parts);
                    let pres_tree = PresentationTree::new(pres, PresTreeData::Rule(pres_tree));
                    let pres_tree = ctx.arenas.presentation_trees.intern(pres_tree);

                    successful_fragments.push((frag_id, pres_tree, mapping));
                }

                // Now pop the bindings we added to the name context.
                names
                    .bindings
                    .truncate(names.bindings.len() - binding_count);
            }
        }
    }

    if successful_fragments.len() == 1 {
        let (frag, pres, mut mapping) = successful_fragments.pop().unwrap();
        mapping.frags.insert(tree, cur_path.clone());
        Ok(Some((frag, pres, mapping)))
    } else {
        Ok(None)
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
