use crate::{
    diagnostics::{DiagManager, WResult},
    parse::parse_tree::ParseTree,
};

pub enum ElaborationError {
    Missing,
    TooDeep,
}

pub fn reduce_to_builtin(tree: ParseTree, diags: &mut DiagManager) -> WResult<ParseTree> {
    let mut node = match tree {
        ParseTree::Atom(_) => return Ok(tree),
        ParseTree::Node(node) => node,
        ParseTree::MacroBinding(_) => todo!(),
    };

    const MAX_DEPTH: usize = 128;
    let mut depth = 0;

    while !node.category.is_builtin() {
        node = node;

        depth += 1;
        if depth >= MAX_DEPTH {
            return diags.err_elaboration_infinite_recursion();
        }
    }

    Ok(ParseTree::Node(node))
}
