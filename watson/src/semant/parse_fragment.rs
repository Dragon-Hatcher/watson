use itertools::chain;
use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{macros::do_macro_replacement, parse_state::ParseRuleSource, parse_tree::{ParseTree, ParseTreeChildren, ParseTreeId, ParseTreePart}},
    semant::{formal_syntax::FormalSyntaxCatId, fragment::FragmentId},
};

pub struct NameCtx {
    templates: FxHashMap<Ustr, TemplateInfo>,
    shorthands: FxHashMap<Ustr, FragmentId>,
}

pub struct TemplateInfo {
    cat: FormalSyntaxCatId,
    args: Vec<FormalSyntaxCatId>,
}

pub fn parse_fragment(
    tree: ParseTreeId,
    expected_cat: FormalSyntaxCatId,
    names: &NameCtx,
    ctx: &mut Ctx,
) -> WResult<FragmentId> {
    // Formal syntax fragments when written in watson source code may be 
    // ambiguous between different formal syntax categories. So we don't parse
    // them until we know which category they should be. At the point this 
    // function is called, we know the expected category.

    // The first thing we are going to do is expand the given parse tree into
    // one that contains only formal syntax rules, i.e. no macros. This will
    // make it possible to perform name resolution correctly by exposing all
    // bindings.
    let expanded_tree = expand_tree(tree, ctx); 

    

    todo!()
}

fn expand_tree(tree: ParseTreeId, ctx: &mut Ctx) -> ParseTreeId {
    // This function takes a parse tree that may contain macros and expands
    // all macros into their definitions, producing a parse tree that only
    // contains formal syntax rules.
    //
    // This is necessary because macros can introduce new bindings and
    // change the structure of the parse tree in ways that affect name
    // resolution and fragment parsing.
    //
    // We are going to expand from the bottom up, so we don't have to expand
    // the same macro multiple times. So the first step is to expand all the
    // children of this node, then if this node is a macro we expand it. The
    // macro might introduce new child macros that need to be expanded so we 
    // repeat until there are no more macros.

    if !ctx.parse_forest.has_unexpanded_macro(tree) {
        return tree;
    }

    let old_tree = &ctx.parse_forest[tree];
    let span = old_tree.span();
    let cat = old_tree.cat();
    
    let mut new_possibilities = Vec::new();
    let mut possibilities_to_expand = old_tree.possibilities().to_vec();

    while let Some(possibility) = possibilities_to_expand.pop() {
        // First expand all children.

        let mut new_children = Vec::with_capacity(possibility.children().len());
        for child in possibility.children() {
            match child {
                ParseTreePart::Atom(atom) => {
                    new_children.push(ParseTreePart::Atom(*atom));
                }
                ParseTreePart::Node { id, span, cat } => {
                    let expanded = expand_tree(*id, ctx);
                    new_children.push(ParseTreePart::Node { id: expanded, span: *span, cat: *cat });
                }
            }
        }

        let possibility = ParseTreeChildren::new(possibility.rule(), new_children);

        // Now the children are clean. We either push this possibility or expand
        // it if it's a macro.

        let rule = &ctx.parse_state[possibility.rule()];

        let &ParseRuleSource::Macro(macro_id) = rule.source() else {
            new_possibilities.push(possibility);
            continue;
        };

        let bindings = &ctx.macros[macro_id].collect_macro_bindings(&possibility);
        let expanded = do_macro_replacement(tree, bindings, ctx);
        
        for possibility in ctx.parse_forest[expanded].possibilities() {
            possibilities_to_expand.push(possibility.clone());
        }
    }

    let new_tree = ParseTree::new(span, cat, new_possibilities);
    ctx.parse_forest.get_or_insert(new_tree)
}

fn disambiguate(tree: ParseTreeId, expected_cat: FormalSyntaxCatId, names: &NameCtx, ctx: &mut Ctx) -> WResult<ParseTreeId> {

}
