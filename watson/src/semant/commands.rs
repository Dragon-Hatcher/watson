use crate::generate_arena_handle;

generate_arena_handle! { CommandId => CommandInfo }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandInfo {}

impl CommandInfo {
    pub fn new() -> Self {
        Self {}
    }
}
