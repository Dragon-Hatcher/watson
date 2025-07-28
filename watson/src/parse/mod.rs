mod earley;
mod elaborator;
mod location;
mod parse_tree;
mod source_cache;

use crate::parse::{
    earley::{parse_category, parse_name},
    elaborator::{ElaborateAction, elaborate},
    parse_tree::{COMMAND_CAT, ParseRule, ParseRuleId, ParseTree},
};
pub use location::{Location, SourceId, SourceOffset, Span};
pub use source_cache::SourceCache;
use std::collections::{HashMap, HashSet};
use ustr::Ustr;

pub fn parse(sources: &SourceCache) {
    for key in sources.source_keys() {
        println!("{:?}", key);
    }
}

struct SourceParseProgress {
    /// How much of the source file have we read so far?
    loc: Location,

    /// Given the current parser state, what keywords can start a command?
    command_starters: HashSet<Ustr>,

    /// What parsing rules have been declared by this file?
    rules: HashMap<ParseRuleId, ParseRule>,

    /// The commands that have been recovered from the source so far. Note that
    /// these have already been elaborated so nothing more needs to be done with
    /// them. But we keep them for reference.
    commands: Vec<ParseTree>,
}

fn make_progress_on_source(progress: &mut SourceParseProgress, sources: &SourceCache) {
    let text = sources.get_text(progress.loc.source());

    while progress.loc.byte_offset() < text.len() {
        // Now we check if the line starts with one of the keywords that
        // signifies a command.
        if let Some((kw, _)) = parse_name(text, progress.loc)
            && progress.command_starters.contains(&kw)
        {
            // Now we can force parsing of a command at this spot in the source.
            let command = parse_category(text, progress.loc, *COMMAND_CAT, &progress.rules);

            // Elaborate the command in our current context.
            match elaborate(command, progress) {
                Ok(ElaborateAction::Continue) => todo!(),
                Ok(ElaborateAction::WaitForSource(id)) => return,
                Err(err) => todo!(),
            }
        } else {
            // We didn't match any of our command starters so this was just prose.
            // We can skip past the rest of this line.
            let rest = &text[progress.loc.byte_offset()..];
            let line = rest.split_inclusive('\n').next().unwrap();
            progress.loc = progress.loc.forward(line.len());
        }
    }
}
