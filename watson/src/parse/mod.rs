mod builtin;
mod earley;
mod elaborator;
pub mod location;
mod macros;
pub mod parse_tree;
pub mod source_cache;

use crate::{
    diagnostics::DiagManager,
    parse::{
        builtin::{
            add_builtin_syntax, add_formal_lang_syntax, add_macro_match_syntax, add_macro_syntax, elaborate_command, COMMAND_CAT
        },
        earley::{find_start_keywords, parse_category, parse_name},
        location::SourceOffset,
        macros::Macros,
        parse_tree::{ParseRule, ParseRuleId, ParseTree, SyntaxCategoryId},
    },
    semant::{formal_syntax::{FormalSyntax, FormalSyntaxCatId}, theorem::Theorems},
    strings,
};
pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;
use std::collections::{HashMap, HashSet, VecDeque};
use ustr::Ustr;

pub fn parse(root: SourceId, sources: &mut SourceCache, diags: &mut DiagManager) {
    let mut progress = SourceParseProgress {
        categories: HashMap::new(),
        rules: HashMap::new(),
        command_starters: HashSet::new(),
        formal_syntax: FormalSyntax::new(),
        macros: Macros::new(),
        theorems: Theorems::new(),
        commands: Vec::new(),
        next_sources: VecDeque::new(),
    };

    progress
        .formal_syntax
        .add_cat(FormalSyntaxCatId::new(*strings::SENTENCE));

    progress.build_parser_state();
    progress.next_sources.push_back(root);

    while let Some(next) = progress.next_sources.pop_front() {
        parse_source(next, &mut progress, sources, diags);
    }
}

struct SourceParseProgress {
    /// The grammar categories from our grammar so far.
    categories: HashMap<Ustr, SyntaxCategoryId>,

    /// What parsing rules have been declared?
    rules: HashMap<ParseRuleId, ParseRule>,

    /// Given the current parsing rules, what keywords can start a command?
    command_starters: HashSet<Ustr>,

    /// The syntax of the formal language.
    formal_syntax: FormalSyntax,

    /// The macros we have found so far.
    macros: Macros,

    /// The theorems we have seen
    theorems: Theorems,

    /// The commands that have been recovered from the source so far. Note that
    /// these have already been elaborated so nothing more needs to be done with
    /// them. But we keep them for reference.
    commands: Vec<ParseTree>,

    /// The sources that we are going to parse and elaborate next.
    next_sources: VecDeque<SourceId>,
}

impl SourceParseProgress {
    fn add_rule(&mut self, rule: ParseRule) {
        self.categories.insert(rule.cat.name(), rule.cat);
        self.rules.insert(rule.id, rule);
    }

    fn build_parser_state(&mut self) {
        add_builtin_syntax(self);
        add_formal_lang_syntax(self);
        add_macro_syntax(self);

        for (_name, cat) in self.categories.clone() {
            // self.categories.insert(k, v)
            add_macro_match_syntax(cat, self);
        }

        self.command_starters = find_start_keywords(*COMMAND_CAT, &self.rules);
    }
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
            let command = parse_category(
                text,
                loc,
                None,
                *COMMAND_CAT,
                &progress.rules,
                None,
                false,
                diags,
            );

            // Now we can skip the command we just parsed. If we didn't manage
            // to parse anything then we skip to the next line below.
            if let Some(command) = command {
                loc = command.span().end();

                // Elaborate the command in our current context.
                elaborate_command(command, progress, sources, diags);
                progress.build_parser_state();

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
