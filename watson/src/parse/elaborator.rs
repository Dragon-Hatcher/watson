use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceCache, SourceParseProgress,
        parse_tree::{MACRO_RULE, MODULE_RULE, ParseTree},
    },
    strings,
};

pub fn elaborate(
    command: &ParseTree,
    progress: &mut SourceParseProgress,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) -> WResult<()> {
    let builtin = reduce_to_builtin(command, diags)?;

    if let Some(children) = builtin.as_rule(*MODULE_RULE) {
        // We should add the declared module to the list of sources to be
        // examined after this source is complete.
        assert!(children[0].is_kw(*strings::MODULE));
        let name = children[1].as_name().unwrap();
        assert!(children.len() == 2);
        
        Ok(ElaborateAction::WaitForSource(source))
    } else if let Some(children) = builtin.as_rule(*MACRO_RULE) {
        todo!()
    } else {
        unreachable!("No elaborator for {:?}.", builtin);
    }
}



pub enum ElaborationError {
    Missing,
    TooDeep,
}

fn reduce_to_builtin(tree: &ParseTree, diags: &mut DiagManager) -> WResult<ParseTree> {
    todo!()
}
