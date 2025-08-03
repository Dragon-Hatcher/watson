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
        ParseTree::Atom(_) | ParseTree::Missing(_) => return Ok(tree),
        ParseTree::Node(node) => node,
    };

    const MAX_DEPTH: usize = 128;
    let mut depth = 0;

    while !node.category.is_builtin() {
        node = node;

        depth += 1;
        if depth >= MAX_DEPTH {
            // TODO: Real error
            return Err(());
        }
    }

    Ok(ParseTree::Node(node))
}
