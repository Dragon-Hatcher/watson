use crate::parse::{
    SourceId, SourceParseProgress,
    parse_tree::{IMPORT_RULE, MACRO_RULE, ParseTree},
};

pub enum ElaborateAction {
    Continue,
    WaitForSource(SourceId),
}

pub enum ElaborationError {
    Missing,
    TooDeep,
}

pub fn elaborate(
    command: ParseTree,
    progress: &mut SourceParseProgress,
) -> Result<ElaborateAction, ElaborationError> {
    let builtin = reduce_to_builtin(command)?;

    if let Some(children) = builtin.as_rule(*IMPORT_RULE) {
        // We need to wait on a source to be completed first so we can import it.
        let source = todo!();

        Ok(ElaborateAction::WaitForSource(source))
    } else if let Some(children) = builtin.as_rule(*MACRO_RULE) {
        todo!()
    } else {
        unreachable!("No elaborator for {:?}.", builtin);
    }
}

fn reduce_to_builtin(tree: ParseTree) -> Result<ParseTree, ElaborationError> {
    todo!()
}
