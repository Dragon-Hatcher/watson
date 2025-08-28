mod earley;
pub mod elaborator;
pub mod grammar;
pub mod location;
pub mod macros;
pub mod parse_state;
pub mod parse_tree;
pub mod source_cache;

pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;

use crate::context::Ctx;

pub fn parse(root: SourceId, ctx: &mut Ctx) {
    let mut sources_stack = Vec::new();
    sources_stack.push(root.start_loc());

    while let Some(next) = sources_stack.pop() {
        parse_source(next, ctx, &mut sources_stack);
    }
}

#[allow(unused)]
fn parse_source(loc: Location, ctx: &mut Ctx, sources_stack: &mut Vec<Location>) {
    let source = loc.source();
    let text = ctx.sources.get_text(source);

    if loc.byte_offset() >= text.len() {
        // This file is finished so we don't need to do anything more.
        return;
    }

    // Check if the current line could possible start a command.
    if true {
        // The current line could start a command so we will assume it does.

        let Ok(promise) = earley::parse(loc, ctx.builtin_cats.command, ctx) else {
            // We weren't able to parse a command. The parse error has already
            // been reported so we will simply move onto the next line.
            let text = ctx.sources.get_text(source);
            sources_stack.push(next_line(text, loc));
            return;
        };

        // Push the location after this command onto the stack so we can
        // continue parsing this source file later.
        let after_command = promise.span().end();
        sources_stack.push(after_command);

        // Now let's elaborate the command.
        let Ok(new_source) = elaborator::elaborate_command(promise, ctx) else {
            // There was an error elaborating the command. We don't need to do
            // anything more here as the error has already been reported.
            return;
        };

        if let Some(new_source) = new_source {
            // This command was a module declaration so we need to parse the
            // newly loaded source file as well. Pushing to the stack now
            // means we will parse it before continuing with the current file.
            let start_loc = new_source.start_loc();
            sources_stack.push(start_loc);
        }
    } else {
        // This line doesn't start a command so we can skip to the next line.
        let text = ctx.sources.get_text(source);
        sources_stack.push(next_line(text, loc));
    }
}

fn next_line(text: &str, loc: Location) -> Location {
    let rest = &text[loc.byte_offset()..];
    if let Some(line) = rest.split_inclusive('\n').next() {
        loc.forward(line.len())
    } else {
        loc.forward(rest.len())
    }
}
