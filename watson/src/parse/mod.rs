pub mod builtin;
mod earley;
pub mod elaborator;
pub mod location;
pub mod macros;
pub mod parse_tree;
pub mod source_cache;

use crate::{
    context::Ctx,
    diagnostics::DiagManager,
    parse::{
        builtin::{
            COMMAND_CAT, add_builtin_syntax, add_formal_lang_syntax, add_macro_match_syntax,
            add_macro_syntax, elaborate_command,
        },
        earley::{find_start_keywords, parse_category, parse_name},
        location::SourceOffset,
        macros::Macros,
        parse_tree::{CategoryId, ParseTree, Rule, RuleId},
    },
    semant::{
        formal_syntax::{FormalSyntax, FormalSyntaxCatId},
        theorem::TheoremId,
        unresolved::UnresolvedTheorem,
    },
    strings,
};
pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    vec,
};
use ustr::Ustr;

pub fn parse(root: SourceId, ctx: &mut Ctx) {
    ctx.formal_syntax
        .add_cat(FormalSyntaxCatId::new(*strings::SENTENCE));

    let mut sources_queue = VecDeque::new();
    sources_queue.push_back(root);

    while let Some(next) = sources_queue.pop_front() {
        parse_source(next, ctx);
    }
}

// impl SourceParseProgress {
//     fn add_rule(&mut self, rule: ParseRule) {
//         self.categories.insert(rule.cat.name(), rule.cat);
//         self.rules.insert(rule.id, rule);
//     }

//     fn build_parser_state(&mut self) {
//         add_builtin_syntax(self);
//         add_formal_lang_syntax(self);
//         add_macro_syntax(self);

//         for (_name, cat) in self.categories.clone() {
//             // self.categories.insert(k, v)
//             add_macro_match_syntax(cat, self);
//         }

//         self.command_starters = find_start_keywords(*COMMAND_CAT, &self.rules);
//     }
// }

#[allow(unused)]
fn parse_source(source: SourceId, ctx: &mut Ctx) {
    let mut loc = source.start_loc();
    let source_length = ctx.sources.get_text(source).len();

    while loc.byte_offset() < source_length {
        let text = ctx.sources.get_text(source);

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

        let text = ctx.sources.get_text(source);

        // We didn't match any of our command starters so this was just prose.
        // We can skip past the rest of this line.
        let rest = &text[loc.byte_offset()..];
        let line = rest.split_inclusive('\n').next().unwrap();
        loc = loc.forward(line.len());
    }
}
