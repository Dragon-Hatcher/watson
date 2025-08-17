use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{
        macros::Macros,
        parse_tree::{ParseNode, ParseRuleId, ParseTree},
    },
};
use std::collections::HashMap;
use ustr::Ustr;

pub fn reduce_to_builtin(
    mut tree: ParseTree,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<ParseTree> {
    let span = tree.span();

    const MAX_DEPTH: usize = 128;
    let mut depth = 0;

    while let ParseTree::Node(node) = &tree
        && let ParseRuleId::Macro(macro_id) = node.rule
    {
        let macro_info = macros.get(macro_id).unwrap();
        tree = replace_bindings(
            macro_info.replacement().clone(),
            node,
            macro_info.pat().keys(),
        );

        depth += 1;
        if depth >= MAX_DEPTH {
            return diags.err_elaboration_infinite_recursion(span);
        }
    }

    Ok(tree)
}

fn replace_bindings(tree: ParseTree, source: &ParseNode, keys: &HashMap<Ustr, usize>) -> ParseTree {
    match tree {
        ParseTree::Atom(atom) => ParseTree::Atom(atom),
        ParseTree::Node(node) => ParseTree::Node(ParseNode {
            children: node
                .children
                .into_iter()
                .map(|child| replace_bindings(child, source, keys))
                .collect(),
            ..node
        }),
        ParseTree::MacroBinding(binding) => {
            let pos = keys[&binding.name];
            source.children[pos].clone()
        }
    }
}
