mod builtin;
mod earley;
mod elaborator;
mod location;
mod parse_tree;
mod source_cache;

use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{
        builtin::{COMMAND_CAT, add_builtin_syntax, elaborate_command},
        earley::{find_start_keywords, parse_category, parse_name},
        location::SourceOffset,
        parse_tree::{ParseRule, ParseRuleId, ParseTree},
    },
    strings,
};
pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;
use std::collections::{HashMap, HashSet, VecDeque};
use ustr::Ustr;

pub fn parse(root: SourceId, sources: &mut SourceCache, diags: &mut DiagManager) {
    let mut progress = SourceParseProgress {
        rules: HashMap::new(),
        command_starters: HashSet::new(),
        formal_syntax_categories: HashSet::new(),
        commands: Vec::new(),
        next_sources: VecDeque::new(),
    };

    progress.formal_syntax_categories.insert(*strings::SENTENCE);

    add_builtin_syntax(&mut progress.rules);
    progress.command_starters = find_start_keywords(*COMMAND_CAT, &progress.rules);
    progress.next_sources.push_back(root);

    while let Some(next) = progress.next_sources.pop_front() {
        dbg!(next);
        parse_source(next, &mut progress, sources, diags);
    }
}

struct SourceParseProgress {
    /// What parsing rules have been declared?
    rules: HashMap<ParseRuleId, ParseRule>,

    /// Given the current parsing rules, what keywords can start a command?
    command_starters: HashSet<Ustr>,

    /// The syntactic categories of the formal language.
    formal_syntax_categories: HashSet<Ustr>,

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

            dbg!(&command);

            let mut skipped = false;
            // Now we can skip the command we just parsed. If we didn't manage
            // to parse anything then we skip to the next line below.
            if !command.is_missing() {
                loc = command.span().end();
                skipped = true;
            } else {
                // We continue past the parse failure.
                let _: WResult<()> = diags.err_parse_failure();
            }

            // Elaborate the command in our current context.
            let command = elaborate_command(command, progress, sources, diags);

            if skipped {
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
