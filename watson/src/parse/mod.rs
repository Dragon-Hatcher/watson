mod earley;
mod elaborator;
mod location;
mod parse_tree;
mod source_cache;

use crate::{
    diagnostics::DiagManager,
    parse::{
        earley::{parse_category, parse_name},
        elaborator::elaborate,
        location::SourceOffset,
        parse_tree::{COMMAND_CAT, ParseRule, ParseRuleId, ParseTree},
    },
};
pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;
use std::collections::{HashMap, HashSet, VecDeque};
use ustr::Ustr;

pub fn parse(sources: &SourceCache) {
    for key in sources.source_keys() {
        println!("{:?}", key);
    }
}

struct SourceParseProgress {
    /// Given the current parser state, what keywords can start a command?
    command_starters: HashSet<Ustr>,

    /// What parsing rules have been declared?
    rules: HashMap<ParseRuleId, ParseRule>,

    /// The commands that have been recovered from the source so far. Note that
    /// these have already been elaborated so nothing more needs to be done with
    /// them. But we keep them for reference.
    commands: Vec<ParseTree>,

    /// The sources that we are going to parse and elaborate next.
    next_sources: VecDeque<SourceId>,
}

#[allow(unused)]
fn parse_source(
    source: SourceId,
    progress: &mut SourceParseProgress,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) {
    let mut loc = Location::new(source, SourceOffset::new(0));

    while loc.byte_offset() < sources.get_text(source).len() {
        let text = sources.get_text(source);

        // Now we check if the line starts with one of the keywords that
        // signifies a command.
        if let Some((kw, _)) = parse_name(text, loc)
            && progress.command_starters.contains(&kw)
        {
            // Now we can force parsing of a command at this spot in the source.
            let command = parse_category(text, loc, *COMMAND_CAT, &progress.rules);

            // Elaborate the command in our current context.
            elaborate(&command, progress, sources, diags);

            // Now we can skip the command we just parsed. If we didn't manage
            // to parse anything then we skip to the next line below.
            if !command.is_missing() {
                loc = command.span().end();
                continue;
            }
        }

        let text = sources.get_text(source);

        // We didn't match any of our command starters so this was just prose.
        // We can skip past the rest of this line.
        let rest = &text[loc.byte_offset()..];
        let line = rest.split_inclusive('\n').next().unwrap();
        loc = loc.forward(line.len());
    }
}
