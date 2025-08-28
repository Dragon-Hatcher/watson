use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        SourceId,
        parse_state::ParseRuleSource,
        parse_tree::{ParseTreeId, ParseTreePromise},
        source_cache::{SourceDecl, source_id_to_path},
    },
    semant::formal_syntax::FormalSyntaxCat,
    strings,
};

pub fn elaborate_command(command: ParseTreePromise, ctx: &mut Ctx) -> WResult<Option<SourceId>> {
    let command_id = resolve_promise(command, ctx)?;
    let command_id = reduce_to_builtin(command_id, ctx)?;

    if ctx.parse_forest[command_id].rule() == ctx.builtin_rules.mod_command {
        let source_id = elaborate_module(command_id, ctx)?;
        return Ok(Some(source_id));
    } else if ctx.parse_forest[command_id].rule() == ctx.builtin_rules.syntax_cat_command {
        elaborate_syntax_cat(command_id, ctx)?;
    } else {
        failed_to_match_builtin();
    }

    Ok(None)
}

macro_rules! extract_children {
    ($tree:expr => $( $name:ident ),*) => {
        let [ $( $name ),* ] = $tree.children() else {
            failed_to_match_builtin();
        };
        $(
            let $name = *$name;
        )*
    };
}

fn elaborate_module(module: ParseTreeId, ctx: &mut Ctx) -> WResult<SourceId> {
    // command ::= kw"module" name
    extract_children!(ctx.parse_forest[module] => module_kw, source_id_name);

    debug_assert!(module_kw.is_kw(*strings::MODULE));
    let source_id = SourceId::new(source_id_name.as_name().unwrap());

    if ctx.sources.has_source(source_id) {
        return ctx.diags.err_module_redeclaration(
            source_id,
            source_id_name.span(),
            ctx.sources.get_decl(source_id),
        );
    }

    let path = source_id_to_path(source_id, ctx.sources.root_dir());
    let Ok(text) = std::fs::read_to_string(&path) else {
        return ctx
            .diags
            .err_non_existent_file(&path, source_id_name.span());
    };

    ctx.sources
        .add(source_id, text, SourceDecl::Module(source_id_name.span()));

    Ok(source_id)
}

fn elaborate_syntax_cat(cat: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // command ::= kw"syntax_cat" name
    extract_children!(ctx.parse_forest[cat] => syntax_kw, cat_name);

    debug_assert!(syntax_kw.is_kw(*strings::SYNTAX));
    let cat_name = cat_name.as_name().unwrap();

    if ctx.formal_syntax.cat_by_name(cat_name).is_some() {
        return ctx.diags.err_duplicate_formal_syntax_cat();
    }

    ctx.formal_syntax.add_cat(FormalSyntaxCat::new(cat_name));
    Ok(())
}

fn resolve_promise(promise: ParseTreePromise, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    let rules = ctx.parse_forest.rules_for_promise(promise);
    match rules {
        [] => panic!("No rules matched promise."),
        [rule] => Ok(ctx.parse_forest.resolve_promise(promise, *rule)),
        _ => ctx.diags.err_ambiguous_parse(promise.span()),
    }
}

fn reduce_to_builtin(tree_id: ParseTreeId, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    loop {
        let tree = &ctx.parse_forest[tree_id];
        let rule = &ctx.parse_state[tree.rule()];

        let ParseRuleSource::Macro(_) = rule.source() else {
            return Ok(tree_id);
        };

        todo!("Macro expansion not yet implemented.");
    }
}

fn failed_to_match_builtin() -> ! {
    panic!("Failed to match builtin parse tree.");
}
