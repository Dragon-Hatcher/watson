mod earley;
pub mod elaborator;
pub mod grammar;
pub mod location;
pub mod parse_state;
pub mod parse_tree;
pub mod source_cache;

pub use location::{Location, SourceId, Span};
pub use source_cache::SourceCache;
use ustr::Ustr;

use crate::{
    context::Ctx,
    parse::{
        earley::parse_name,
        elaborator::ElaborateAction,
        parse_state::{
            Associativity, Category, ParseAtomPattern, Precedence, SyntaxCategorySource,
        },
        parse_tree::ParseTreeId,
    },
    semant::{
        formal_syntax::FormalSyntaxCatId,
        notation::{NotationPattern, NotationPatternPart},
        scope::Scope,
        tactic::unresolved_proof::UnresolvedProof,
        theorems::TheoremId,
    },
};

pub struct ParseReport<'ctx> {
    pub theorems: Vec<(TheoremId<'ctx>, UnresolvedProof<'ctx>)>,
    pub entries: Vec<ParseEntry<'ctx>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseEntry<'ctx> {
    Text(Span),
    Command(ParseTreeId<'ctx>),
}

pub fn parse<'ctx>(root: SourceId, ctx: &mut Ctx<'ctx>) -> ParseReport<'ctx> {
    let mut sources_stack = Vec::new();
    let mut scope = Scope::new();
    sources_stack.push(root.start_loc());

    let mut theorems = Vec::new();
    let mut entries = Vec::new();
    while let Some(next) = sources_stack.pop() {
        parse_source(
            next,
            ctx,
            &mut sources_stack,
            &mut scope,
            &mut theorems,
            &mut entries,
        );
    }

    ParseReport { theorems, entries }
}

fn parse_source<'ctx>(
    loc: Location,
    ctx: &mut Ctx<'ctx>,
    sources_stack: &mut Vec<Location>,
    scope: &mut Scope<'ctx>,
    theorems: &mut Vec<(TheoremId<'ctx>, UnresolvedProof<'ctx>)>,
    entries: &mut Vec<ParseEntry<'ctx>>,
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

        let tree = match earley::parse(loc, ctx.builtin_cats.command, ctx) {
            Ok(tree) => tree,
            Err(diags) => {
                // We weren't able to parse a command. Add the diagnostics and
                // move onto the next line.
                ctx.diags.add_diags(diags);
                sources_stack.push(next_line(text, loc));
                return;
            }
        };

        entries.push(ParseEntry::Command(tree));

        // Push the location after this command onto the stack so we can
        // continue parsing this source file later.
        let after_command = tree.span().end();
        sources_stack.push(after_command);

        // Now let's elaborate the command.
        let action = match elaborator::elaborate_command(tree, scope, ctx) {
            Ok(action) => action,
            Err(diags) => {
                // There was an error elaborating the command. Add the diagnostics
                // and continue.
                ctx.diags.add_diags(diags);
                return;
            }
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

                add_formal_cat(cat, ctx);
            }
            ElaborateAction::NewFormalRule(rule) => {
                // The command created a new formal syntax rule. We need to
                // update the state of the parser to include this rule.
                let (pattern, binding, scope_entry) = grammar::formal_rule_to_notation(rule, ctx);
                grammar::add_parse_rules_for_notation(pattern, ctx);
                ctx.parse_state.recompute_initial_atoms();

                *scope = scope.child_with(binding, scope_entry);
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
            ElaborateAction::NewTheorem(new_theorem, proof) => {
                theorems.push((new_theorem, proof));
            }
            ElaborateAction::NewTacticCat(cat) => {
                // The command created a new tactic category. We need to
                // update the state of the parser to include this category.
                let parse_cat = Category::new(cat.name(), SyntaxCategorySource::Tactic(cat));
                let parse_cat = ctx.arenas.parse_cats.alloc(cat.name(), parse_cat);
                ctx.parse_state.use_cat(parse_cat);
                ctx.parse_state.recompute_initial_atoms();

                ctx.tactic_manager.use_tactic_cat(cat);
            }
            ElaborateAction::NewTacticRule(rule) => {
                // The command created a new tactic rule. We need to
                // update the state of the parser to include this rule.
                grammar::add_parse_rules_for_tactic_rule(rule, ctx);
                ctx.parse_state.recompute_initial_atoms();

                ctx.tactic_manager.use_tactic_rule(rule);
            }
        }
    } else {
        // This line doesn't start a command so we can skip to the next line.
        let next_loc = next_line(text, loc);
        sources_stack.push(next_loc);

        if let Some(ParseEntry::Text(prev_span)) = entries.last()
            && prev_span.end() == loc
        {
            // We can merge this text span with the previous one.
            let span = Span::new(prev_span.start(), next_loc);
            entries.pop();
            entries.push(ParseEntry::Text(span));
        } else {
            let span = Span::new(loc, next_loc);
            entries.push(ParseEntry::Text(span));
        }
    }
}

pub fn add_formal_cat<'ctx>(cat: FormalSyntaxCatId<'ctx>, ctx: &mut Ctx<'ctx>) {
    grammar::add_parse_rules_for_formal_cat(cat, ctx);

    // We also add a notation for this category which is just a
    // single name. This is needed to allow bindings an also just
    // for convenience.
    let name = Ustr::from(&format!("{}.name", cat.name()));
    let parts = vec![NotationPatternPart::Name];
    let prec = Precedence::default();
    let assoc = Associativity::default();
    let notation = NotationPattern::new(name, cat, parts, prec, assoc);
    let notation = ctx.arenas.notations.alloc(notation);
    grammar::add_parse_rules_for_notation(notation, ctx);

    ctx.single_name_notations.insert(cat, notation);

    // Update the parse state given all the rules we have added.
    ctx.parse_state.recompute_initial_atoms();
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
