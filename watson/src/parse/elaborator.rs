use crate::{
    context::Ctx,
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceId,
        parse_state::{ParseRuleSource, RuleId},
        parse_tree::{ParseForest, ParseTreeChildren, ParseTreeId},
        source_cache::{SourceDecl, source_id_to_path},
    },
    semant::formal_syntax::{
        FormalSyntaxCat, FormalSyntaxPat, FormalSyntaxPatPart, FormalSyntaxRule,
    },
    strings,
};

pub fn elaborate_command(command: ParseTreeId, ctx: &mut Ctx) -> WResult<Option<SourceId>> {
    let command = reduce_to_builtin(command, ctx)?;
    let children = expect_unambiguous(command, &ctx.parse_forest, &mut ctx.diags)?;

    if children.rule() == ctx.builtin_rules.mod_command {
        let new_source = elaborate_module(command, ctx)?;
        return Ok(Some(new_source));
    } else if children.rule() == ctx.builtin_rules.syntax_cat_command {
        elaborate_syntax_cat(command, ctx)?;
    } else if children.rule() == ctx.builtin_rules.syntax_command {
        elaborate_syntax(command, ctx)?;
    } else {
        failed_to_match_builtin(children.rule(), ctx);
    }

    Ok(None)
}

macro_rules! match_rule {
    (($ctx:expr, $tree_id:expr) => $($rule:ident ::= [$($child:ident),*] => $body:expr),+ $(,)?) => {{
        let tree = reduce_to_builtin($tree_id, $ctx)?;
        let children = expect_unambiguous(tree, &$ctx.parse_forest, &mut $ctx.diags)?;
        $(
            if children.rule() == $ctx.builtin_rules.$rule {
                let [$($child),*] = children.children() else {
                    failed_to_match_builtin(children.rule(), $ctx);
                };
                $(
                    let $child = *$child;
                )*
                $body
            } else
        )*
        {
            failed_to_match_builtin(children.rule(), $ctx);
        }
    }}
}

fn elaborate_module(module: ParseTreeId, ctx: &mut Ctx) -> WResult<SourceId> {
    // command ::= kw"module" name

    match_rule! { (ctx, module) =>
        mod_command ::= [module_kw, source_id_name] => {
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

    }
}

fn elaborate_syntax_cat(cat: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // command ::= kw"syntax_cat" name

    match_rule! { (ctx, cat) =>
        syntax_cat_command ::= [syntax_kw, cat_name] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX_CAT));
            let cat_name = cat_name.as_name().unwrap();

            if ctx.formal_syntax.cat_by_name(cat_name).is_some() {
                return ctx.diags.err_duplicate_formal_syntax_cat();
            }

            ctx.formal_syntax.add_cat(FormalSyntaxCat::new(cat_name));
            Ok(())
        }
    }
}

fn elaborate_syntax(syntax: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // command ::= kw"syntax" name name "::=" syntax_pat_list kw"end"

    match_rule! { (ctx, syntax) =>
        syntax_command ::= [syntax_kw, rule_name, cat, bnf_replace, pat_list, end_kw] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = rule_name.as_name().unwrap();
            let cat_name = cat.as_name().unwrap();
            let pat = elaborate_syntax_pat_list(pat_list.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat();
            };

            let rule = FormalSyntaxRule::new(rule_name, cat, pat);
            ctx.formal_syntax.add_rule(rule);

            Ok(())
        }
    }
}

fn elaborate_syntax_pat_list(mut pat_list: ParseTreeId, ctx: &mut Ctx) -> WResult<FormalSyntaxPat> {
    // syntax_pat_list ::= (syntax_pat_list_one)  syntax_pat
    //                   | (syntax_pat_list_many) syntax_pat syntax_pat_list

    let mut parts = Vec::new();
    loop {
        match_rule! { (ctx, pat_list) =>
            syntax_pat_list_one ::= [pat] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_syntax_pat(pat, ctx)?);
                break;
            },
            syntax_pat_list_many ::= [pat, rest] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_syntax_pat(pat, ctx)?);
                pat_list = rest.as_node().unwrap();
            }
        }
    }

    let pat = FormalSyntaxPat::new(parts);
    Ok(pat)
}

fn elaborate_syntax_pat(pat: ParseTreeId, ctx: &mut Ctx) -> WResult<FormalSyntaxPatPart> {
    // syntax_pat ::= (syntax_pat_cat)     name
    //              | (syntax_pat_binding) "@" kw"binding" "(" name ")"
    //              | (syntax_pat_var)     "@" kw"variable" "(" name ")"
    //              | (syntax_pat_lit)     str

    match_rule! { (ctx, pat) =>
        syntax_pat_cat ::= [cat_name] => {
            let cat_name = cat_name.as_name().unwrap();

            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat();
            };

            Ok(FormalSyntaxPatPart::Cat(cat))
        },
        syntax_pat_binding ::= [at, binding_kw, l_paren, name, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(binding_kw.is_kw(*strings::BINDING));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let name = name.as_name().unwrap();
            Ok(FormalSyntaxPatPart::Binding(name))
        },
        syntax_pat_var ::= [at, var_kw, l_paren, name, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(var_kw.is_kw(*strings::VARIABLE));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let name = name.as_name().unwrap();
            Ok(FormalSyntaxPatPart::Var(name))
        },
        syntax_pat_lit ::= [lit] => {
            let lit = lit.as_str_lit().unwrap();
            Ok(FormalSyntaxPatPart::Lit(lit))
        }
    }
}

fn expect_unambiguous<'a>(
    id: ParseTreeId,
    forest: &'a ParseForest,
    diags: &mut DiagManager,
) -> WResult<&'a ParseTreeChildren> {
    match forest[id].possibilities() {
        [] => unreachable!("No possibilities in parse tree."),
        [possibility] => Ok(possibility),
        _ => diags.err_ambiguous_parse(forest[id].span()),
    }
}

fn reduce_to_builtin(tree_id: ParseTreeId, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    let tree = &ctx.parse_forest[tree_id];

    for possibility in tree.possibilities() {
        let rule = &ctx.parse_state[possibility.rule()];

        let ParseRuleSource::Macro(_) = rule.source() else {
            continue;
        };

        todo!("Macro expansion not yet implemented.");
    }

    Ok(tree_id)
}

fn failed_to_match_builtin(rule: RuleId, ctx: &Ctx) -> ! {
    panic!(
        "Failed to match builtin parse tree: {}",
        ctx.parse_state[rule].name()
    );
}
