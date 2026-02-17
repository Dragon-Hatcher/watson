use crate::semant::{commands::CommandId, custom_grammar::inst::CustomGrammarInst};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<'ctx>(pub CustomGrammarInst<'ctx>);

pub struct AttributeTracker<'ctx> {
    attrs: im::HashMap<CommandId<'ctx>, Vec<Attribute<'ctx>>>,
}

impl<'ctx> AttributeTracker<'ctx> {
    pub fn new() -> Self {
        Self {
            attrs: im::HashMap::new(),
        }
    }

    pub fn child_with(&self, cmd: CommandId<'ctx>, attrs: Vec<Attribute<'ctx>>) -> Self {
        let mut map = self.attrs.clone();
        map.insert(cmd, attrs);
        Self { attrs: map }
    }
}
