mod earley;
pub mod elaborator;
pub mod grammar;
pub mod location;
pub mod parse_state;
pub mod parse_tree;
pub mod source_cache;

pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;

use crate::{
    context::Ctx,
    parse::{earley::parse_name, elaborator::ElaborateAction, parse_state::{Category, ParseAtomPattern, SyntaxCategorySource}},
    semant::{scope::Scope, theorems::TheoremId},
};

pub fn parse<'ctx>(root: SourceId, ctx: &mut Ctx<'ctx>) -> Vec<TheoremId<'ctx>> {
    let mut sources_stack = Vec::new();
    let mut scope = Scope::new();
    sources_stack.push(root.start_loc());

    let mut theorems = Vec::new();
    while let Some(next) = sources_stack.pop() {
        parse_source(next, ctx, &mut sources_stack, &mut scope, &mut theorems);
    }

    theorems
}
fn parse_source<'ctx>(
    loc: Location,
    ctx: &mut Ctx<'ctx>,
    sources_stack: &mut Vec<Location>,
    scope: &mut Scope<'ctx>,
    theorems: &mut Vec<TheoremId<'ctx>>,
) {
    let source = loc.source();
    let text = ctx.sources.get_text(source).as_str();

    if loc.byte_offset() >= text.len() {
        // This file is finished so we don't need to do anything more.
        return;
    }

    // Check if the current line could possible start a command.
    if can_start_command(text, loc, ctx) {
        // The current line could start a command so we will assume it does.

        let Ok(tree) = earley::parse(loc, ctx.builtin_cats.command, ctx) else {
            // We weren't able to parse a command. The parse error has already
            // been reported so we will simply move onto the next line.
            sources_stack.push(next_line(text, loc));
            return;
        };

        // Push the location after this command onto the stack so we can
        // continue parsing this source file later.
        let after_command = tree.span().end();
        sources_stack.push(after_command);

        // Now let's elaborate the command.
        let Ok(action) = elaborator::elaborate_command(tree, &scope, ctx) else {
            // There was an error elaborating the command. We don't need to do
            // anything more here as the error has already been reported.
            return;
        };

        match action {
            ElaborateAction::NewSource(new_source) => {
                // This command was a module declaration so we need to parse the
                // newly loaded source file as well. Pushing to the stack now
                // means we will parse it before continuing with the current file.
                let start_loc = new_source.start_loc();
                sources_stack.push(start_loc);
            }
            ElaborateAction::NewFormalCat(cat) => {
                // The command created a new formal syntax category. We need to
                // update the state of the parser to include this category.

                let parse_cat = Category::new(cat.name(), SyntaxCategorySource::FormalLang(cat)); 
                let parse_cat = ctx.arenas.parse_cats.alloc(cat.name(), parse_cat);
                ctx.parse_state.use_cat(parse_cat);

                grammar::add_parse_rules_for_formal_cat(cat, ctx);
                ctx.parse_state.recompute_initial_atoms();
            }
            ElaborateAction::NewFormalRule(rule) => {
                // The command created a new formal syntax rule. We need to
                // update the state of the parser to include this rule.
                let notation = grammar::formal_rule_to_notation(rule, ctx);
                grammar::add_parse_rules_for_notation(notation, ctx);
                ctx.parse_state.recompute_initial_atoms();
            }
            ElaborateAction::NewNotation(notation) => {
                // The command created new notation. We need to update the state
                // of the parser to include this notation.
                grammar::add_parse_rules_for_notation(notation, ctx);
                ctx.parse_state.recompute_initial_atoms();
            }
            ElaborateAction::NewDefinition(new_scope) => {
                // The definition added a new binding to the scope. Replace the
                // old scope with the new one.
                *scope = new_scope;
            }
            ElaborateAction::NewTheorem(new_theorem) => {
                theorems.push(new_theorem);
            }
            ElaborateAction::None => {}
        }
    } else {
        // This line doesn't start a command so we can skip to the next line.
        sources_stack.push(next_line(text, loc));
    }
}

fn can_start_command(text: &str, loc: Location, ctx: &Ctx) -> bool {
    let name = parse_name(text, loc.offset());

    for atom in ctx.parse_state.initial_atoms(ctx.builtin_cats.command) {
        match atom {
            ParseAtomPattern::Lit(lit) => {
                if text[loc.byte_offset()..].starts_with(lit.as_str()) {
                    return true;
                }
            }
            ParseAtomPattern::Kw(kw) => {
                if let Some((_, parsed_name)) = name
                    && parsed_name == kw.as_str()
                {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

fn next_line(text: &str, loc: Location) -> Location {
    let rest = &text[loc.byte_offset()..];
    if let Some(line) = rest.split_inclusive('\n').next() {
        loc.forward(line.len())
    } else {
        loc.forward(rest.len())
    }
}
