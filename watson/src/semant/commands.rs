use crate::generate_arena_handle;

generate_arena_handle! { CommandId => CommandInfo }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandInfo {
    // prevent Rust from treating this struct as a constant.
    _data: i32,
}

impl CommandInfo {
    pub fn new() -> Self {
        Self { _data: 0 }
    }
}
