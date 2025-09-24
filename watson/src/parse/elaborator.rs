use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::{
    context::{self, Ctx},
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceId,
        grammar::{self, add_builtin_syntax_for_formal_cat},
        macros::{Macro, MacroPat, do_macro_replacement},
        parse_state::{ParseAtomPattern, ParseRuleSource, Rule, RulePatternPart},
        parse_tree::{
            _debug_parse_tree, ParseForest, ParseTree, ParseTreeChildren, ParseTreeId,
            ParseTreePart,
        },
        source_cache::{SourceDecl, source_id_to_path},
    },
    semant::{
        check_proof::{UnresolvedByTactic, UnresolvedHaveTactic, UnresolvedTactic},
        formal_syntax::{
            FormalSyntaxCat, FormalSyntaxCatId, FormalSyntaxPat, FormalSyntaxPatPart,
            FormalSyntaxRule,
        },
        parse_fragment::{NameCtx, UnresolvedFact, parse_fact, parse_fragment},
        theorems::{Template, TheoremId, TheoremStatement, UnresolvedProof},
    },
    strings,
};

macro_rules! failed_to_match_builtin {
    ($rule:expr, $ctx:expr) => {
        panic!(
            "Failed to match builtin parse tree: {}",
            $ctx.parse_state[$rule].name()
        );
    };
}

macro_rules! match_rule {
    (($ctx:expr, $tree_id:expr) => $($rule:ident ::= [$($child:ident),*] => $body:expr),+ $(,)?) => {{
        debug_unambiguous($tree_id, $ctx);
        let tree = reduce_to_builtin($tree_id, $ctx)?;
        let children = expect_unambiguous(tree, &$ctx.parse_forest, &mut $ctx.diags)?;
        $(
            if children.rule() == $ctx.builtin_rules.$rule {
                let [$($child),*] = children.children() else {
                    failed_to_match_builtin!(children.rule(), $ctx);
                };
                $(
                    let $child = *$child;
                )*
                $body
            } else
        )*
        {
            failed_to_match_builtin!(children.rule(), $ctx);
        }
    }}
}

pub fn elaborate_command(command: ParseTreeId, ctx: &mut Ctx) -> WResult<Option<SourceId>> {
    // command ::= (module_command)     module_command
    //           | (syntax_cat_command) syntax_cat_command
    //           | (syntax_command)     syntax_command
    //           | (macro_command)      macro_command
    //           | (axiom_command)      axiom_command
    //           | (theorem_command)    theorem_command

    match_rule! { (ctx, command) =>
        module_command ::= [module_cmd] => {
            let new_source = elaborate_module(module_cmd.as_node().unwrap(), ctx)?;
            Ok(Some(new_source))
        },
        syntax_cat_command ::= [cat_cmd] => {
            elaborate_syntax_cat(cat_cmd.as_node().unwrap(), ctx)?;
            Ok(None)
        },
        syntax_command ::= [syntax_cmd] => {
            elaborate_syntax(syntax_cmd.as_node().unwrap(), ctx)?;
            Ok(None)
        },
        macro_command ::= [macro_cmd] => {
            elaborate_macro(macro_cmd.as_node().unwrap(), ctx)?;
            Ok(None)
        },
        axiom_command ::= [axiom_cmd] => {
            elaborate_axiom(axiom_cmd.as_node().unwrap(), ctx)?;
            Ok(None)
        },
        theorem_command ::= [theorem_cmd] => {
            elaborate_theorem(theorem_cmd.as_node().unwrap(), ctx)?;
            Ok(None)
        },
    }
}

fn elaborate_module(module: ParseTreeId, ctx: &mut Ctx) -> WResult<SourceId> {
    // module_command ::= (module) kw"module" name

    match_rule! { (ctx, module) =>
        module ::= [module_kw, source_id_name] => {
            debug_assert!(module_kw.is_kw(*strings::MODULE));
            let source_id_str = elaborate_name(source_id_name.as_node().unwrap(), ctx)?;
            let source_id = SourceId::new(source_id_str);

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
    // syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name

    match_rule! { (ctx, cat) =>
        syntax_cat ::= [syntax_kw, cat_name] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX_CAT));
            let cat_name = elaborate_name(cat_name.as_node().unwrap(), ctx)?;

            if ctx.formal_syntax.cat_by_name(cat_name).is_some() {
                return ctx.diags.err_duplicate_formal_syntax_cat();
            }

            let formal_cat = ctx.formal_syntax.add_cat(FormalSyntaxCat::new(cat_name));

            ctx.parse_state.new_formal_lang_cat(cat_name, formal_cat);

            add_builtin_syntax_for_formal_cat(formal_cat, ctx);

            Ok(())
        }
    }
}

fn elaborate_syntax(syntax: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // syntax_command ::= (syntax) kw"syntax" name name "::=" syntax_pat_list kw"end"

    match_rule! { (ctx, syntax) =>
        syntax ::= [syntax_kw, rule_name, cat, bnf_replace, pat_list, end_kw] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let pat = elaborate_syntax_pat(pat_list.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat.span());
            };

            if let Some(_) = ctx.formal_syntax.rule_by_name(rule_name) {
                return ctx.diags.err_duplicate_formal_syntax_rule();
            }

            let rule = FormalSyntaxRule::new(rule_name, cat, pat);
            let rule_id = ctx.formal_syntax.add_rule(rule);

            grammar::add_formal_syntax_rule(rule_id, ctx);

            Ok(())
        }
    }
}

fn elaborate_syntax_pat(mut pat_list: ParseTreeId, ctx: &mut Ctx) -> WResult<FormalSyntaxPat> {
    // syntax_pat ::= (syntax_pat_one)  syntax_pat_part
    //              | (syntax_pat_many) syntax_pat_part syntax_pat

    let mut parts = Vec::new();

    loop {
        match_rule! { (ctx, pat_list) =>
            syntax_pat_one ::= [pat] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_syntax_pat_part(pat, ctx)?);
                break;
            },
            syntax_pat_many ::= [pat, rest] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_syntax_pat_part(pat, ctx)?);
                pat_list = rest.as_node().unwrap();
            }
        }
    }

    let pat = FormalSyntaxPat::new(parts);
    Ok(pat)
}

fn elaborate_syntax_pat_part(pat: ParseTreeId, ctx: &mut Ctx) -> WResult<FormalSyntaxPatPart> {
    // syntax_pat_part ::= (syntax_pat_cat)     name
    //                   | (syntax_pat_binding) "@" kw"binding" "(" name ")"
    //                   | (syntax_pat_var)     "@" kw"variable" "(" name ")"
    //                   | (syntax_pat_lit)     str

    match_rule! { (ctx, pat) =>
        syntax_pat_part_cat ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(FormalSyntaxPatPart::Cat(cat))
        },
        syntax_pat_part_binding ::= [at, binding_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(binding_kw.is_kw(*strings::BINDING));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(FormalSyntaxPatPart::Binding(cat))
        },
        syntax_pat_part_var ::= [at, var_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(var_kw.is_kw(*strings::VARIABLE));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(FormalSyntaxPatPart::Var(cat))
        },
        syntax_pat_part_lit ::= [lit] => {
            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(FormalSyntaxPatPart::Lit(lit))
        }
    }
}

fn elaborate_macro(macro_cmd: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // macro_command ::= (macro) kw"macro" name macro_replacement kw"end"

    match_rule! { (ctx, macro_cmd) =>
        macro_r ::= [macro_kw, macro_name, replacement, end_kw] => {
            debug_assert!(macro_kw.is_kw(*strings::MACRO));
            debug_assert!(end_kw.is_kw(*strings::END));

            let macro_name = elaborate_name(macro_name.as_node().unwrap(), ctx)?;
            let replacement = replacement.as_node().unwrap();

            let (pat, replacement) = elaborate_macro_replacement(replacement, ctx)?;
            let cat = ctx.parse_forest[replacement].cat();

            let Some(replacement) = disambiguate_macro_replacement(replacement, &pat, ctx)? else {
                // TODO: correct error type.
                return ctx.diags.err_ambiguous_parse(ctx.parse_forest[replacement].span());
            };
            let macro_id = ctx.macros.add_macro(Macro::new(macro_name, cat, pat, replacement));

            let rule_pat = ctx.macros[macro_id].pat().to_parse_rule();
            let rule = Rule::new(macro_name, cat, ParseRuleSource::Macro(macro_id), rule_pat);
            ctx.parse_state.add_rule(rule);

            Ok(())
        }
    }
}

fn disambiguate_macro_replacement(
    replacement: ParseTreeId,
    pat: &MacroPat,
    ctx: &mut Ctx,
) -> WResult<Option<ParseTreeId>> {
    fn check_possibility(
        replacement: ParseTreeId,
        possibility: ParseTreeChildren,
        pat: &MacroPat,
        ctx: &mut Ctx,
    ) -> WResult<ParseTreeChildren> {
        if let [child] = possibility.children()
            && let Some(binding) = child.as_macro_binding()
        {
            // This rule is a macro binding so we have to check that it matches
            // the given pattern.

            let Some(idx) = pat.keys().get(&binding) else {
                return ctx
                    .diags
                    .err_undefined_macro_binding(binding, ctx.parse_forest[replacement].span());
            };

            let part = &pat.parts()[*idx];

            if let RulePatternPart::Cat {
                id: expected_cat, ..
            } = part
                && ctx.parse_forest[replacement].cat() != *expected_cat
            {
                // The category of this binding doesn't match the expected
                // category from the pattern.
                return Err(());
            }

            Ok(possibility)
        } else {
            // Otherwise we just need to recursively check all the children of this
            // possibility.
            let mut new_children = Vec::new();
            let mut err = false;

            for child in possibility.children() {
                let new_child = match child {
                    ParseTreePart::Atom(atom) => ParseTreePart::Atom(*atom),
                    ParseTreePart::Node { id, span, cat } => {
                        let Ok(Some(new_child)) = disambiguate_macro_replacement(*id, pat, ctx)
                        else {
                            err = true;
                            continue;
                        };

                        ParseTreePart::Node {
                            id: new_child,
                            span: *span,
                            cat: *cat,
                        }
                    }
                };
                new_children.push(new_child);
            }

            if err {
                return Err(());
            }

            Ok(ParseTreeChildren::new(possibility.rule(), new_children))
        }
    }

    let mut new_possibilities = Vec::new();

    let old_tree = &ctx.parse_forest[replacement];
    let span = old_tree.span();
    let cat = old_tree.cat();

    for possibility in old_tree.possibilities().to_owned() {
        let Ok(new_possibility) = check_possibility(replacement, possibility, pat, ctx) else {
            continue;
        };

        new_possibilities.push(new_possibility);
    }

    if new_possibilities.is_empty() {
        return Ok(None);
    }

    let tree = ParseTree::new(span, cat, new_possibilities);
    Ok(Some(ctx.parse_forest.get_or_insert(tree)))
}

fn elaborate_macro_replacement(
    replacement: ParseTreeId,
    ctx: &mut Ctx,
) -> WResult<(MacroPat, ParseTreeId)> {
    // macro_replacement ::= (macro_replacement) <category> "::=" macro_pat_list "=>" template(category)

    let replacement = reduce_to_builtin(replacement, ctx)?;
    let children = expect_unambiguous(replacement, &ctx.parse_forest, &mut ctx.diags)?;

    let [_cat_name, bnf_replace, pat, arrow, template] = children.children() else {
        failed_to_match_builtin!(children.rule(), ctx);
    };

    debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
    debug_assert!(arrow.is_lit(*strings::FAT_ARROW));

    let template = template.as_node().unwrap();
    let pat = elaborate_macro_pat(pat.as_node().unwrap(), ctx)?;

    Ok((pat, template))
}

fn elaborate_macro_pat(mut pat: ParseTreeId, ctx: &mut Ctx) -> WResult<MacroPat> {
    // macro_pat ::= (macro_pat_one)  macro_pat_part
    //             | (macro_pat_many) macro_pat_part macro_pat

    let mut parts = Vec::new();
    let mut keys = FxHashMap::default();

    loop {
        let (name, pat_part, rest) = match_rule! { (ctx, pat) =>
            macro_pat_one ::= [part] => {
                let part = part.as_node().unwrap();
                let (name, pat_part) = elaborate_macro_pat_part(part, ctx)?;
                (name, pat_part, None)
            },
            macro_pat_many ::= [part, rest] => {
                let part = part.as_node().unwrap();
                let rest = rest.as_node().unwrap();
                let (name, pat_part) = elaborate_macro_pat_part(part, ctx)?;
                (name, pat_part, Some(rest))
            }
        };

        parts.push(pat_part);

        if let Some(name) = name {
            if keys.contains_key(&name) {
                return ctx.diags.err_duplicate_macro_binding();
            }

            keys.insert(name, parts.len() - 1);
        }

        match rest {
            Some(rest) => {
                pat = rest;
            }
            None => {
                break;
            }
        }
    }

    Ok(MacroPat::new(parts, keys))
}

fn elaborate_macro_pat_part(
    pat: ParseTreeId,
    ctx: &mut Ctx,
) -> WResult<(Option<Ustr>, RulePatternPart)> {
    // macro_pat_part ::= (macro_pat_part) macro_pat_binding macro_pat_kind

    match_rule! { (ctx, pat) =>
        macro_pat_part ::= [binding, kind] => {
            let binding = binding.as_node().unwrap();
            let kind = kind.as_node().unwrap();

            let name = elaborate_macro_binding(binding, ctx)?;
            let kind = elaborate_macro_pat_kind(kind, ctx)?;

            Ok((name, kind))
        }
    }
}

fn elaborate_macro_binding(binding: ParseTreeId, ctx: &mut Ctx) -> WResult<Option<Ustr>> {
    // macro_pat_binding ::= (macro_pat_binding_empty)
    //                     | (macro_pat_binding_name)  "$" name ":"

    match_rule! { (ctx, binding) =>
        macro_pat_binding_empty ::= [] => {
            Ok(None)
        },
        macro_pat_binding_name ::= [dollar, name, colon] => {
            debug_assert!(dollar.is_lit(*strings::DOLLAR));
            debug_assert!(colon.is_lit(*strings::COLON));

            let name = elaborate_name(name.as_node().unwrap(), ctx)?;
            Ok(Some(name))
        }
    }
}

fn elaborate_macro_pat_kind(pat: ParseTreeId, ctx: &mut Ctx) -> WResult<RulePatternPart> {
    // macro_pat_kind ::= (macro_pat_kind_kw)       "@" kw"kw" str
    //                  | (macro_pat_kind_lit)      str
    //                  | (macro_pat_kind_cat)      name
    //                  | (macro_pat_kind_template) "@" kw"template" "(" name ")"

    match_rule! { (ctx, pat) =>
        macro_pat_kind_kw ::= [at, kw_kw, lit] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(kw_kw.is_kw(*strings::KW));

            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(RulePatternPart::Atom(ParseAtomPattern::Kw(lit)))
        },
        macro_pat_kind_lit ::= [lit] => {
            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(RulePatternPart::Atom(ParseAtomPattern::Lit(lit)))
        },
        macro_pat_kind_cat ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.parse_state.cat_by_name(cat_name) else {
                return ctx.diags.err_non_existent_syntax_category(cat_name, cat_name_node.span());
            };

            Ok(RulePatternPart::Cat { id: cat, template: false })
        },
        macro_pat_kind_template ::= [at, template_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(template_kw.is_kw(*strings::TEMPLATE));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.parse_state.cat_by_name(cat_name) else {
                return ctx.diags.err_non_existent_syntax_category(cat_name, cat_name_node.span());
            };

            Ok(RulePatternPart::Cat { id: cat, template: true })
        }

    }
}

fn elaborate_axiom(axiom: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"

    match_rule! { (ctx, axiom) =>
        axiom ::= [axiom_kw, name_node, templates, colon, hypotheses, turnstile, sentence, end_kw] => {
            debug_assert!(axiom_kw.is_kw(*strings::AXIOM));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let name = elaborate_name(name_node.as_node().unwrap(), ctx)?;
            let templates = elaborate_templates(templates.as_node().unwrap(), ctx)?;

            let mut name_ctx = make_name_ctx(&templates);
            let hypotheses = elaborate_hypotheses(hypotheses.as_node().unwrap(), ctx)?;
            let hypotheses = hypotheses.into_iter().map(|h| parse_fact(h, &mut name_ctx, ctx)).collect::<WResult<Vec<_>>>()?;
            let conclusion = parse_fragment(sentence.as_node().unwrap(), ctx.formal_syntax.sentence_cat(), &mut name_ctx, ctx)?;

            let theorem_stmt = TheoremStatement::new(templates, hypotheses, conclusion, UnresolvedProof::Axiom);

            let id = TheoremId::new(name);

            if let Some(_previous) = ctx.theorem_stmts.get(id) {
                return ctx.diags.err_duplicate_theorem(name, name_node.span());
            }

            ctx.theorem_stmts.add(id, theorem_stmt);

            Ok(())
        }
    }
}

fn elaborate_theorem(theorem: ParseTreeId, ctx: &mut Ctx) -> WResult<()> {
    // theorem_command ::= (theorem) kw"theorem" name templates ":" hypotheses "|-" sentence kw"proof" tactic kw"qed"

    match_rule! { (ctx, theorem) =>
        theorem ::= [theorem_kw, name_node, templates, colon, hypotheses, turnstile, sentence, proof_kw, tactic, qed_kw] => {
            debug_assert!(theorem_kw.is_kw(*strings::THEOREM));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));
            debug_assert!(proof_kw.is_kw(*strings::PROOF));
            debug_assert!(qed_kw.is_kw(*strings::QED));

            let name = elaborate_name(name_node.as_node().unwrap(), ctx)?;
            let templates = elaborate_templates(templates.as_node().unwrap(), ctx)?;

            let mut name_ctx = make_name_ctx(&templates);
            let hypotheses = elaborate_hypotheses(hypotheses.as_node().unwrap(), ctx)?;
            let hypotheses = hypotheses.into_iter().map(|h| parse_fact(h, &mut name_ctx, ctx)).collect::<WResult<Vec<_>>>()?;
            let conclusion = parse_fragment(sentence.as_node().unwrap(), ctx.formal_syntax.sentence_cat(), &mut name_ctx, ctx)?;

            let tactic = tactic.as_node().unwrap();
            let theorem_stmt = TheoremStatement::new(templates, hypotheses, conclusion, UnresolvedProof::Theorem(tactic));

            let id = TheoremId::new(name);

            if let Some(_previous) = ctx.theorem_stmts.get(id) {
                return ctx.diags.err_duplicate_theorem(name, name_node.span());
            }

            ctx.theorem_stmts.add(id, theorem_stmt);

            Ok(())
        }
    }
}

fn make_name_ctx(templates: &[Template]) -> NameCtx {
    let mut names = NameCtx::new();
    for template in templates {
        names.add_template(template.name(), template.clone());
    }
    names
}

fn elaborate_templates(mut templates: ParseTreeId, ctx: &mut Ctx) -> WResult<Vec<Template>> {
    // templates ::= (template_none)
    //             | (template_many) template templates

    let mut templates_list = Vec::new();

    loop {
        match_rule! { (ctx, templates) =>
            template_none ::= [] => {
                return Ok(templates_list);
            },
            template_many ::= [template, rest] => {
                let template = template.as_node().unwrap();

                templates_list.push(elaborate_template(template, ctx)?);
                templates = rest.as_node().unwrap();
            }
        }
    }
}

fn elaborate_template(template: ParseTreeId, ctx: &mut Ctx) -> WResult<Template> {
    // template ::= (template) "[" name maybe_template_params ":" name "]"

    match_rule! { (ctx, template) =>
        template ::= [l_brack, name, maybe_params, colon, cat_name_node, r_brack] => {
            debug_assert!(l_brack.is_lit(*strings::LEFT_BRACKET));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(r_brack.is_lit(*strings::RIGHT_BRACKET));

            let name = elaborate_name(name.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };
            let params = elaborate_maybe_template_params(maybe_params.as_node().unwrap(), ctx)?;

            Ok(Template::new(name, cat, params))
        }
    }
}

fn elaborate_maybe_template_params(
    maybe_params: ParseTreeId,
    ctx: &mut Ctx,
) -> WResult<Vec<FormalSyntaxCatId>> {
    // maybe_template_params ::= (maybe_template_params_none)
    //                         | (maybe_template_params_some) "(" template_params ")"

    match_rule! { (ctx, maybe_params) =>
        maybe_template_params_none ::= [] => {
            Ok(Vec::new())
        },
        maybe_template_params_some ::= [l_paren, params, r_paren] => {
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            elaborate_template_params(params.as_node().unwrap(), ctx)
        }
    }
}

fn elaborate_template_params(
    params: ParseTreeId,
    ctx: &mut Ctx,
) -> WResult<Vec<FormalSyntaxCatId>> {
    // template_params ::= (template_params_one)  template_param
    //                   | (template_params_many) template_param "," template_params

    let mut params_list = Vec::new();
    let mut next_params = Some(params);

    while let Some(params) = next_params {
        let (param, rest) = match_rule! { (ctx, params) =>
            template_params_one ::= [param] => {
                let param = param.as_node().unwrap();
                (param, None)
            },
            template_params_many ::= [param, comma, rest] => {
                let param = param.as_node().unwrap();
                let rest = rest.as_node().unwrap();
                debug_assert!(comma.is_lit(*strings::COMMA));

                (param, Some(rest))
            }
        };

        params_list.push(elaborate_template_param(param, ctx)?);
        next_params = rest;
    }

    Ok(params_list)
}

fn elaborate_template_param(param: ParseTreeId, ctx: &mut Ctx) -> WResult<FormalSyntaxCatId> {
    // template_param ::= (template_param) name

    match_rule! { (ctx, param) =>
        template_param ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.formal_syntax.cat_by_name(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(cat)
        }
    }
}

fn elaborate_hypotheses(hypotheses: ParseTreeId, ctx: &mut Ctx) -> WResult<Vec<UnresolvedFact>> {
    // hypotheses ::= (hypotheses_none)
    //              | (hypotheses_many) hypothesis hypotheses

    let mut hypotheses_list = Vec::new();
    let mut next_hypotheses = Some(hypotheses);

    while let Some(hypotheses) = next_hypotheses {
        match_rule! { (ctx, hypotheses) =>
            hypotheses_none ::= [] => {
                next_hypotheses = None;
            },
            hypotheses_many ::= [hypothesis, rest] => {
                let hypothesis = hypothesis.as_node().unwrap();
                let rest = rest.as_node().unwrap();

                hypotheses_list.push(elaborate_hypothesis(hypothesis, ctx)?);
                next_hypotheses = Some(rest);
            }
        }
    }

    Ok(hypotheses_list)
}

fn elaborate_hypothesis(hypothesis: ParseTreeId, ctx: &mut Ctx) -> WResult<UnresolvedFact> {
    // hypothesis ::= (hypothesis) "(" fact ")"

    match_rule! { (ctx, hypothesis) =>
        hypothesis ::= [l_paren, fact, r_paren] => {
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let fact = fact.as_node().unwrap();
            let fact = elaborate_fact(fact, ctx)?;

            Ok(fact)
        }
    }
}

fn elaborate_fact(fact: ParseTreeId, ctx: &mut Ctx) -> WResult<UnresolvedFact> {
    // fact ::= (fact_assumption) kw"assume" sentence "|-" sentence
    //        | (fact_sentence)   sentence

    match_rule! { (ctx, fact) =>
        fact_assumption ::= [assume_kw, assumption, turnstile, conclusion] => {
            debug_assert!(assume_kw.is_kw(*strings::ASSUME));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));

            let assumption = assumption.as_node().unwrap();
            let conclusion = conclusion.as_node().unwrap();
            Ok(UnresolvedFact::new(Some(assumption), conclusion))
        },
        fact_sentence ::= [conclusion] => {
            let conclusion = conclusion.as_node().unwrap();
            Ok(UnresolvedFact::new(None, conclusion))
        }
    }
}

pub fn elaborate_maybe_shorthand_args(
    maybe_args: ParseTreeId,
    ctx: &mut Ctx,
) -> WResult<Vec<ParseTreeId>> {
    // maybe_shorthand_args ::= (maybe_shorthand_args_none)
    //                        | (maybe_shorthand_args_some) "(" shorthand_args ")"

    match_rule! { (ctx, maybe_args) =>
        maybe_shorthand_args_none ::= [] => {
            Ok(Vec::new())
        },
        maybe_shorthand_args_some ::= [l_paren, args, r_paren] => {
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            elaborate_shorthand_args(args.as_node().unwrap(), ctx)
        }
    }
}

fn elaborate_shorthand_args(args: ParseTreeId, ctx: &mut Ctx) -> WResult<Vec<ParseTreeId>> {
    let mut next_args = Some(args);
    let mut arg_list = Vec::new();

    while let Some(args) = next_args {
        // shorthand_args ::= (shorthand_args_one)  shorthand_arg
        //                  | (shorthand_args_many) shorthand_arg "," shorthand_args

        let (arg, next) = match_rule! { (ctx, args) =>
            shorthand_args_one ::= [arg] => {
                let arg = arg.as_node().unwrap();
                (arg, None)
            },
            shorthand_args_many ::= [arg, comma, rest] => {
                let arg = arg.as_node().unwrap();
                let rest = rest.as_node().unwrap();
                debug_assert!(comma.is_lit(*strings::COMMA));
                (arg, Some(rest))
            }
        };

        arg_list.push(elaborate_shorthand_arg(arg, ctx)?);
        next_args = next;
    }

    Ok(arg_list)
}

fn elaborate_shorthand_arg(arg: ParseTreeId, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    // shorthand_arg ::= (shorthand_arg) any_fragment

    match_rule! { (ctx, arg) =>
        shorthand_arg ::= [frag] => {
            let frag = frag.as_node().unwrap();
            debug_assert!(ctx.parse_forest[frag].cat() == ctx.builtin_cats.any_fragment);

            Ok(frag)
        }
    }
}

pub fn elaborate_tactic(tactic: ParseTreeId, ctx: &mut Ctx) -> WResult<UnresolvedTactic> {
    // tactic ::= (tactic_none)
    //          | (tactic_have) kw"have" fact tactics ";" tactics
    //          | (tactic_by)   kw"by" name template_instantiations
    //          | (tactic_todo) kw"todo"

    match_rule! { (ctx, tactic) =>
        tactic_none ::= [] => {
            Ok(UnresolvedTactic::None)
        },
        tactic_have ::= [have_kw, fact, proof, semicolon, continuation] => {
            debug_assert!(have_kw.is_kw(*strings::HAVE));
            debug_assert!(semicolon.is_lit(*strings::SEMICOLON));

            let fact = elaborate_fact(fact.as_node( ).unwrap(), ctx)?;
            let tactic = UnresolvedHaveTactic { fact, proof: proof.as_node().unwrap(), continuation: continuation.as_node().unwrap() };
            Ok(UnresolvedTactic::Have(tactic))
        },
        tactic_by ::= [by_kw, theorem_name_node, template_insts] => {
            debug_assert!(by_kw.is_kw(*strings::BY));

            let theorem_name = elaborate_name(theorem_name_node.as_node().unwrap(), ctx)?;
            let templates = elaborate_template_instantiations(template_insts.as_node().unwrap(), ctx)?;

            let tactic = UnresolvedByTactic { theorem_name, theorem_name_span: theorem_name_node.span(), templates };
            Ok(UnresolvedTactic::By(tactic))
        },
        tactic_todo ::= [todo_kw] => {
            debug_assert!(todo_kw.is_kw(*strings::TODO));

            Ok(UnresolvedTactic::Todo)
        }
    }
}

fn elaborate_template_instantiations(
    mut insts: ParseTreeId,
    ctx: &mut Ctx,
) -> WResult<Vec<ParseTreeId>> {
    // template_instantiations ::= (template_instantiations_none)
    //                           | (template_instantiations_many) template_instantiation template_instantiations

    let mut insts_list = Vec::new();

    loop {
        match_rule! { (ctx, insts) =>
            template_instantiations_none ::= [] => {
                return Ok(insts_list);
            },
            template_instantiations_many ::= [inst, rest] => {
                let inst = inst.as_node().unwrap();
                insts_list.push(elaborate_template_instantiation(inst, ctx)?);
                insts = rest.as_node().unwrap();
            }
        }
    }
}

fn elaborate_template_instantiation(inst: ParseTreeId, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    // template_instantiation ::= (template_instantiation) "[" any_fragment "]"

    match_rule! { (ctx, inst) =>
        template_instantiation ::= [l_brack, frag, r_brack] => {
            debug_assert!(l_brack.is_lit(*strings::LEFT_BRACKET));
            debug_assert!(r_brack.is_lit(*strings::RIGHT_BRACKET));

            let frag = frag.as_node().unwrap();
            debug_assert!(ctx.parse_forest[frag].cat() == ctx.builtin_cats.any_fragment);

            Ok(frag)
        }
    }
}

pub fn elaborate_name(name: ParseTreeId, ctx: &mut Ctx) -> WResult<Ustr> {
    match_rule! { (ctx, name) =>
        name ::= [name_atom] => {
            let name = name_atom.as_name().unwrap();
            Ok(name)
        }
    }
}

fn elaborate_str_lit(str_lit: ParseTreeId, ctx: &mut Ctx) -> WResult<Ustr> {
    match_rule! { (ctx, str_lit) =>
        str ::= [str_atom] => {
            let str_lit = str_atom.as_str_lit().unwrap();
            Ok(str_lit)
        }
    }
}

fn debug_unambiguous(id: ParseTreeId, ctx: &Ctx) {
    let forest = &ctx.parse_forest;
    match forest[id].possibilities() {
        [] => unreachable!("No possibilities in parse tree."),
        [_possibility] => {}
        _ => {
            _debug_parse_tree(id, ctx);
            panic!("Expected unambiguous parse tree.");
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

pub fn reduce_to_builtin(og_tree_id: ParseTreeId, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    let tree = &ctx.parse_forest[og_tree_id];

    // If we have no macro possibilities then we are already done.
    if tree.possibilities().iter().all(|p| {
        let rule = &ctx.parse_state[p.rule()];
        !matches!(rule.source(), ParseRuleSource::Macro(_))
    }) {
        return Ok(og_tree_id);
    }

    // Otherwise we collect all the possibilities into one new parse tree.
    let span = tree.span();
    let cat = tree.cat();
    let mut possibilities = Vec::new();

    for possibility in tree.possibilities().to_owned() {
        let rule = &ctx.parse_state[possibility.rule()];

        let ParseRuleSource::Macro(macro_id) = rule.source() else {
            possibilities.push(possibility.clone());
            continue;
        };

        let macro_ = &ctx.macros[*macro_id];
        let bindings = macro_.collect_macro_bindings(&possibility);
        let tree_id = do_macro_replacement(macro_.replacement(), &bindings, ctx);

        for new_possibility in ctx.parse_forest[tree_id].possibilities() {
            possibilities.push(new_possibility.clone());
        }
    }

    let new_tree = ParseTree::new(span, cat, possibilities);
    let new_tree = ctx.parse_forest.get_or_insert(new_tree);

    Ok(new_tree)
}
