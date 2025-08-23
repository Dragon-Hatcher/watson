#![allow(clippy::let_unit_value)]

use super::macros::MacroPatPart;
use crate::{
    category_id,
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceCache, SourceId, SourceParseProgress, Span,
        earley::parse_category,
        elaborator::reduce_to_builtin,
        macros::{MacroId, MacroInfo, MacroPat, Macros},
        parse_tree::{
            AtomPattern, ParseAtomKind, ParseNode, ParseRule, ParseRuleId, ParseTree, PatternPart,
            SyntaxCategoryId,
        },
    },
    rule_id,
    semant::{
        formal_syntax::{
            FormalSyntax, FormalSyntaxCatId, FormalSyntaxPattern, FormalSyntaxPatternPart,
            FormalSyntaxRule, FormalSyntaxRuleId,
        },
        theorem::{Template, TheoremId},
        unresolved::{
            UnresolvedFact, UnresolvedFragPart, UnresolvedFragment, UnresolvedFragmentData,
            UnresolvedProof, UnresolvedTheorem,
        },
    },
    strings::{self, MACRO_RULE},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    fs,
};
use ustr::Ustr;

category_id!(COMMAND_CAT = "command");

pub(super) fn elaborate_command(
    command: ParseTree,
    progress: &mut SourceParseProgress,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) -> WResult<()> {
    let command = reduce_to_builtin(command, &progress.macros, diags)?;

    if command.as_rule(*MODULE_RULE).is_some() {
        let new_source = elaborate_module(command, sources, diags)?;
        progress.next_sources.push_back(new_source);

        Ok(())
    } else if command.as_rule(*SYNTAX_CAT_RULE).is_some() {
        elaborate_syntax_cat(command, progress, diags)?;
        Ok(())
    } else if command.as_rule(*SYNTAX_RULE).is_some() {
        elaborate_syntax(command, progress, diags)?;
        Ok(())
    } else if command.as_rule_pat(*MACRO_RULE).is_some() {
        elaborate_macro(command, sources, progress, diags)?;
        Ok(())
    } else if command.as_rule(*AXIOM_RULE).is_some() {
        elaborate_axiom(command, progress, diags)?;
        Ok(())
    } else if command.as_rule(*THEOREM_RULE).is_some() {
        elaborate_theorem(command, progress, diags)?;
        Ok(())
    } else {
        unreachable!("No elaborator for parse tree {:?}.", dbg!(command));
    }
}

rule_id!(MODULE_RULE = "module");

fn elaborate_module(
    module: ParseTree,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) -> WResult<SourceId> {
    // module ::= "module" path:name

    let Some([module_kw, path]) = module.as_rule(*MODULE_RULE) else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(module_kw.is_kw(*strings::MODULE));
    let path_str = path.as_name().unwrap();

    // The name takes the form path.from.root so let's parse it like that.
    let mut path = sources.root_dir().to_path_buf();
    for part in path_str.split('.') {
        path.push(part);
    }
    path.set_extension(*strings::FILE_EXTENSION);

    // Now let's check if that module already exists and try to load it.
    let source_id = SourceId::new(path_str);

    // It isn't allowed to load the same file twice.
    if sources.has_source(source_id) {
        return diags.err_module_redeclaration(
            source_id,
            module.span(),
            sources.get_decl(source_id),
        );
    }

    let Ok(source_text) = fs::read_to_string(&path) else {
        return diags.err_non_existent_file(&path, &module);
    };

    sources.add(source_id, source_text, Some(module.span()));

    Ok(source_id)
}

rule_id!(SYNTAX_CAT_RULE = "syntax_cat");

fn elaborate_syntax_cat(
    command: ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<()> {
    // syntax_cat ::= "syntax_category" name

    let Some([syntax_cat_kw, name]) = command.as_rule(*SYNTAX_CAT_RULE) else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(syntax_cat_kw.is_kw(*strings::SYNTAX_CAT));
    let cat = FormalSyntaxCatId::new(name.as_name().unwrap());

    if progress.formal_syntax.has_cat(cat) {
        return diags.err_duplicate_formal_syntax_cat();
    }

    progress.formal_syntax.add_cat(cat);

    Ok(())
}

rule_id!(SYNTAX_RULE = "syntax");

fn elaborate_syntax(
    command: ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<()> {
    // syntax ::= "syntax" name $category:name "::=" syntax_pat "end"

    let Some([syntax_kw, name, cat, bnf_sym, pat_list, end_kw]) = command.as_rule(*SYNTAX_RULE)
    else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(syntax_kw.is_kw(*strings::SYNTAX));
    let rule_id = FormalSyntaxRuleId::new(name.as_name().unwrap());
    let cat = FormalSyntaxCatId::new(cat.as_name().unwrap());
    let _ = assert!(bnf_sym.is_lit(*strings::BNF_REPLACE));
    let pat = elaborate_syntax_pat_list(pat_list.clone(), progress, diags);
    let _ = assert!(end_kw.is_kw(*strings::END));

    if progress.formal_syntax.has_rule(rule_id) {
        return diags.err_duplicate_formal_syntax_rule();
    }

    if !progress.formal_syntax.has_cat(cat) {
        return diags.err_unknown_formal_syntax_cat();
    }

    let pat = pat?;

    progress
        .formal_syntax
        .add_rule(FormalSyntaxRule::new(cat, rule_id, pat));

    Ok(())
}

category_id!(SYNTAX_PAT_LIST = "syntax_pat_list");
rule_id!(SYNTAX_PAT_LIST_ONE = "syntax_pat_list_one");
rule_id!(SYNTAX_PAT_LIST_MORE = "syntax_pat_list_more");

fn elaborate_syntax_pat_list(
    mut pat_list: ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<FormalSyntaxPattern> {
    // syntax_pat_list ::= syntax_pat
    //                   | syntax_pat syntax_pat_list

    let mut parts = Vec::new();

    loop {
        let builtin_pat_list = reduce_to_builtin(pat_list, &progress.macros, diags)?;

        if let Some([part]) = builtin_pat_list.as_rule(*SYNTAX_PAT_LIST_ONE) {
            let part = elaborate_syntax_pat(part, progress, diags)?;
            parts.push(part);
            break;
        } else if let Some([part, rest]) = builtin_pat_list.as_rule(*SYNTAX_PAT_LIST_MORE) {
            let part = elaborate_syntax_pat(part, progress, diags)?;
            parts.push(part);
            pat_list = rest.clone();
        } else {
            panic!("Failed to match builtin rule.");
        }
    }

    Ok(FormalSyntaxPattern::new(parts))
}

category_id!(SYNTAX_PAT = "syntax_pat");
rule_id!(SYNTAX_PAT_BINDING = "syntax_pat_binding");
rule_id!(SYNTAX_PAT_VARIABLE = "syntax_pat_variable");
rule_id!(SYNTAX_PAT_NAME = "syntax_pat_name");
rule_id!(SYNTAX_PAT_STR = "syntax_pat_str");

fn elaborate_syntax_pat(
    pat: &ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<FormalSyntaxPatternPart> {
    // syntax_pat ::= name
    //             | "@" "binding" "(" name ")"
    //             | "@" "variable" "(" name ")"
    //             | str

    let pat = reduce_to_builtin(pat.clone(), &progress.macros, diags)?;

    if let Some([at, name, l_paren, cat, r_paren]) = pat.as_rule(*SYNTAX_PAT_VARIABLE) {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(name.is_kw(*strings::VARIABLE));
        let _ = assert!(l_paren.is_lit(*strings::LEFT_PAREN));
        let _ = assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

        let cat_id = FormalSyntaxCatId::new(cat.as_name().unwrap());
        if !progress.formal_syntax.has_cat(cat_id) {
            return diags.err_undefined_macro_binding(cat_id.name(), cat.span());
        }

        Ok(FormalSyntaxPatternPart::Variable(cat_id))
    } else if let Some([at, name, l_paren, cat, r_paren]) = pat.as_rule(*SYNTAX_PAT_BINDING) {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(name.is_kw(*strings::BINDING));
        let _ = assert!(l_paren.is_lit(*strings::LEFT_PAREN));
        let _ = assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

        let cat_id = FormalSyntaxCatId::new(cat.as_name().unwrap());
        if !progress.formal_syntax.has_cat(cat_id) {
            return diags.err_undefined_macro_binding(cat_id.name(), cat.span());
        }

        Ok(FormalSyntaxPatternPart::Binding(cat_id))
    } else if let Some([name]) = pat.as_rule(*SYNTAX_PAT_NAME) {
        let name = name.as_name().unwrap();

        let cat = FormalSyntaxCatId::new(name);
        if !progress.formal_syntax.has_cat(cat) {
            return diags.err_unknown_formal_syntax_cat();
        }

        Ok(FormalSyntaxPatternPart::Cat(cat))
    } else if let Some([str]) = pat.as_rule(*SYNTAX_PAT_STR) {
        let str = str.as_str().unwrap();
        Ok(FormalSyntaxPatternPart::Lit(str))
    } else {
        panic!("Failed to match builtin rule.");
    }
}

fn elaborate_macro(
    mac: ParseTree,
    sources: &SourceCache,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<()> {
    let mac = reduce_to_builtin(mac, &progress.macros, diags)?;

    // command ::= "macro" name $category:name "::=" macro_pat_list "=>" $category "end"

    let Some(
        [
            macro_kw,
            name,
            cat,
            bnf,
            pattern,
            arrow,
            replacement,
            end_kw,
        ],
    ) = mac.as_rule_pat(*MACRO_RULE)
    else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(macro_kw.is_kw(*strings::MACRO));
    let name_str = name.as_name().unwrap();
    let _ = assert!(bnf.is_lit(*strings::BNF_REPLACE));
    let pattern = elaborate_macro_pat_list(pattern.clone(), progress, diags)?;
    let _ = assert!(arrow.is_lit(*strings::FAT_ARROW));
    let _ = assert!(end_kw.is_kw(*strings::END));

    let id = MacroId::new(name_str);

    if progress.macros.has_id(id) {
        todo!()
    }

    if !check_macro_bindings_ok(replacement, &pattern, diags) {
        return Err(());
    }

    let cat = cat.as_atom().unwrap();
    let ParseAtomKind::Kw(cat) = cat.kind else {
        panic!("Failed to match builtin rule.");
    };
    let Some(cat) = progress.categories.get(&cat) else {
        panic!("Failed to match builtin rule.");
    };

    let replacement = reparse_replacement(replacement, &pattern, sources, progress, diags)?;

    progress.macros.add_macro(MacroInfo::new(
        MacroId::new(name_str),
        *cat,
        pattern,
        replacement,
    ));

    Ok(())
}

fn check_macro_bindings_ok(
    replacement: &ParseTree,
    pattern: &MacroPat,
    diags: &mut DiagManager,
) -> bool {
    match replacement {
        ParseTree::Atom(_) => true,
        ParseTree::Node(node) => {
            let mut all_ok = true;
            for child in &node.children {
                if !check_macro_bindings_ok(child, pattern, diags) {
                    all_ok = false;
                }
            }
            all_ok
        }
        ParseTree::MacroBinding(binding) => {
            if !pattern.keys().contains_key(&binding.name) {
                let _: WResult<()> =
                    diags.err_undefined_macro_binding(binding.name, replacement.span());
                return false;
            }

            true
        }
    }
}

fn reparse_replacement(
    replacement: &ParseTree,
    pattern: &MacroPat,
    sources: &SourceCache,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<ParseTree> {
    match replacement {
        ParseTree::Atom(atom) => Ok(ParseTree::Atom(*atom)),
        ParseTree::Node(node) => {
            if node.has_unchecked_bindings {
                // We need to reparse this node.
                let span = replacement.span();
                let text = sources.get_text(span.source());

                let new = parse_category(
                    text,
                    span.start(),
                    Some(span.end()),
                    node.category,
                    &progress.rules,
                    Some(pattern),
                    true,
                    diags,
                )
                .ok_or(())?;

                // TODO: There is no real justification for this but I'm too lazy right now.
                assert!(new.span() == span);
                Ok(new)
            } else {
                let children: Result<Vec<_>, _> = node
                    .children
                    .iter()
                    .map(|child| reparse_replacement(child, pattern, sources, progress, diags))
                    .collect();
                Ok(ParseTree::Node(ParseNode {
                    children: children?,
                    ..*node
                }))
            }
        }
        ParseTree::MacroBinding(_macro_binding_node) => {
            todo!("TODO: should be the same as above but too lazy right now")
        }
    }
}

category_id!(MACRO_PAT_LIST_CAT = "macro_pat_list");
rule_id!(MACRO_PAT_LIST_ONE = "macro_pat_list_one");
rule_id!(MACRO_PAT_LIST_MORE = "macro_pat_list_more");

fn elaborate_macro_pat_list(
    list: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<MacroPat> {
    let mut parts = Vec::new();
    let mut keys = HashMap::new();
    let mut next_list = Some(list);

    while let Some(list) = next_list {
        let builtin_list = reduce_to_builtin(list, &progress.macros, diags)?;

        // macro_pat_list ::= macro_pat
        //                  | macro_pat macro_pat_list

        let pat = if let Some([pat]) = builtin_list.as_rule(*MACRO_PAT_LIST_ONE) {
            next_list = None;
            pat
        } else if let Some([pat, next]) = builtin_list.as_rule(*MACRO_PAT_LIST_MORE) {
            next_list = Some(next.clone());
            pat
        } else {
            panic!("failed to match builtin rule");
        };

        let (name, pat) = elaborate_macro_pat_part(pat.clone(), progress, diags)?;

        if let Some(name) = name {
            if keys.contains_key(&name) {
                todo!()
            }

            keys.insert(name, parts.len());
        }

        parts.push(pat);
    }

    Ok(MacroPat::new(parts, keys))
}

category_id!(MACRO_PAT_PART_CAT = "macro_pat_part");
rule_id!(MACRO_PAT_PART_RULE = "macro_pat_part");

fn elaborate_macro_pat_part(
    part: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<(Option<Ustr>, MacroPatPart)> {
    let part = reduce_to_builtin(part, &progress.macros, diags)?;

    // macro_pat_part ::= macro_pat_binding macro_pat

    let Some([binding, pat]) = part.as_rule(*MACRO_PAT_PART_RULE) else {
        panic!("failed to match builtin rule");
    };

    let binding = elaborate_macro_pat_binding(binding.clone(), progress, diags)?;
    let pat = elaborate_macro_pat(pat.clone(), progress, diags)?;
    Ok((binding, pat))
}

category_id!(MACRO_PAT_BINDING_CAT = "macro_pat_binding");
rule_id!(MACRO_PAT_BINDING_NONE = "macro_pat_binding_none");
rule_id!(MACRO_PAT_BINDING_NAME = "macro_pat_binding_name");

fn elaborate_macro_pat_binding(
    binding: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<Option<Ustr>> {
    let binding = reduce_to_builtin(binding, &progress.macros, diags)?;

    // macro_pat_binding ::= <nothing> | "$" name ":"

    if let Some([]) = binding.as_rule(*MACRO_PAT_BINDING_NONE) {
        Ok(None)
    } else if let Some([dollar, name, colon]) = binding.as_rule(*MACRO_PAT_BINDING_NAME) {
        let _ = assert!(dollar.is_lit(*strings::DOLLAR));
        let name_str = name.as_name().unwrap();
        let _ = assert!(colon.is_lit(*strings::COLON));
        Ok(Some(name_str))
    } else {
        panic!("failed to match builtin rule")
    }
}

category_id!(MACRO_PAT_CAT = "macro_pat");
rule_id!(MACRO_PAT_KW = "macro_pat_kw");
rule_id!(MACRO_PAT_NAME = "macro_pat_name");
rule_id!(MACRO_PAT_STR = "macro_pat_str");
rule_id!(MACRO_PAT_CAT_REF = "macro_pat_cat_ref");
rule_id!(MACRO_PAT_TEMP_CAT_REF = "macro_pat_temp_cat_ref");

fn elaborate_macro_pat(
    pat: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<MacroPatPart> {
    let pat = reduce_to_builtin(pat, &progress.macros, diags)?;

    // macro_pat ::= "@" "kw" str
    //             | "@" "name"
    //             | str
    //             | name

    if let Some([at, kw_kw, str]) = pat.as_rule(*MACRO_PAT_KW) {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(kw_kw.is_kw(*strings::KW));
        let str = str.as_str().unwrap();
        Ok(MacroPatPart::Kw(str))
    } else if let Some([at, name_kw]) = pat.as_rule(*MACRO_PAT_NAME) {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(name_kw.is_kw(*strings::NAME));
        Ok(MacroPatPart::Name)
    } else if let Some([str]) = pat.as_rule(*MACRO_PAT_STR) {
        let str = str.as_str().unwrap();
        Ok(MacroPatPart::Lit(str))
    } else if let Some([name]) = pat.as_rule(*MACRO_PAT_CAT_REF) {
        let name = name.as_name().unwrap();
        if let Some(cat) = progress.categories.get(&name) {
            Ok(MacroPatPart::Cat(*cat))
        } else {
            return diags.err_non_existent_syntax_category(name, pat.span());
        }
    } else if let Some([at, temp_cat, left_paren, name, right_paren]) =
        pat.as_rule(*MACRO_PAT_TEMP_CAT_REF)
    {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(temp_cat.is_kw(*strings::TEMPLATE));
        let _ = assert!(left_paren.is_lit(*strings::LEFT_PAREN));
        let _ = assert!(right_paren.is_lit(*strings::RIGHT_PAREN));
        let name = name.as_name().unwrap();
        if let Some(cat) = progress.categories.get(&name) {
            Ok(MacroPatPart::TempCat(*cat))
        } else {
            return diags.err_non_existent_syntax_category(name, pat.span());
        }
    } else {
        panic!("failed to match builtin rule")
    }
}

// templates ::= <empty> | template templates
// template ::= "[" name maybe_template_params ":" name "]"
// maybe_template_params = <empty> | "(" template_params ")"
// template_params ::= template_param | template_param "," template_params
// template_param ::= name ":" name
// hypotheses ::= <empty> | hypothesis hypotheses
// hypothesis ::= "(" fact ")"
// fact ::= sentence | "assume" sentence "|-" sentence

rule_id!(AXIOM_RULE = "axiom");

fn elaborate_axiom(
    axiom: ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<()> {
    let axiom = reduce_to_builtin(axiom, &progress.macros, diags)?;

    // command ::= "axiom" name templates ":" hypotheses "|-" sentence "end"
    let Some(
        [
            axiom_kw,
            name,
            templates,
            colon,
            hypotheses,
            turnstile,
            conclusion,
            end_kw,
        ],
    ) = axiom.as_rule(*AXIOM_RULE)
    else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(axiom_kw.is_kw(*strings::AXIOM));
    let name_str = name.as_name().unwrap();
    let templates = elaborate_templates(templates.clone(), progress, diags)?;
    let _ = assert!(colon.is_lit(*strings::COLON));
    let hypotheses = elaborate_hypotheses(hypotheses.clone(), progress, diags)?;
    let _ = assert!(turnstile.is_lit(*strings::TURNSTILE));
    let conclusion = elaborate_formal_fragment(
        conclusion.clone(),
        &progress.formal_syntax,
        &progress.macros,
        diags,
    )?;
    let _ = assert!(end_kw.is_kw(*strings::END));

    let id = TheoremId::new(name_str);

    if progress.theorems.contains_key(&id) {
        return diags.err_duplicate_theorem(name_str, name.span());
    }

    let axiom = UnresolvedTheorem::new(
        id,
        templates,
        hypotheses,
        conclusion,
        UnresolvedProof::Axiom,
    );
    progress.theorems.insert(id, axiom);

    Ok(())
}

category_id!(TEMPLATES_CAT = "templates");
rule_id!(TEMPLATES_EMPTY_RULE = "templates_empty");
rule_id!(TEMPLATES_MORE_RULE = "templates_more");

fn elaborate_templates(
    mut templates: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<Vec<Template>> {
    let mut result = Vec::new();

    loop {
        templates = reduce_to_builtin(templates, &progress.macros, diags)?;

        if let Some([]) = templates.as_rule(*TEMPLATES_EMPTY_RULE) {
            break;
        } else if let Some([template, rest]) = templates.as_rule(*TEMPLATES_MORE_RULE) {
            let template = elaborate_template(template.clone(), progress, diags)?;
            result.push(template);
            templates = rest.clone();
        } else {
            panic!("Failed to match builtin rule.");
        }
    }

    let mut seen_names = HashSet::new();
    for template in &result {
        if !seen_names.insert(template.name()) {
            todo!();
            // diags.err_duplicate_template(name, template.span());
        }
    }

    Ok(result)
}

category_id!(TEMPLATE_CAT = "template");
rule_id!(TEMPLATE_RULE = "template");

fn elaborate_template(
    template: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<Template> {
    let template = reduce_to_builtin(template, &progress.macros, diags)?;

    // template ::= "[" name maybe_template_params ":" name "]"
    let Some([left_bracket, name, params, colon, cat_name, right_bracket]) =
        template.as_rule(*TEMPLATE_RULE)
    else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(left_bracket.is_lit(*strings::LEFT_BRACKET));
    let name_str = name.as_name().unwrap();
    let params = elaborate_maybe_template_params(params.clone(), progress, diags)?;
    let _ = assert!(colon.is_lit(*strings::COLON));
    let cat_name_str = cat_name.as_name().unwrap();
    let _ = assert!(right_bracket.is_lit(*strings::RIGHT_BRACKET));

    let cat_id = FormalSyntaxCatId::new(cat_name_str);
    if !progress.formal_syntax.has_cat(cat_id) {
        return diags.err_unknown_formal_syntax_cat();
    }

    Ok(Template::new(name_str, cat_id, params))
}

category_id!(MAYBE_TEMPLATE_PARAMS_CAT = "maybe_template_params");
rule_id!(MAYBE_TEMPLATE_PARAMS_EMPTY_RULE = "maybe_template_params_empty");
rule_id!(MAYBE_TEMPLATE_PARAMS_PARAMS_RULE = "maybe_template_params_params");

fn elaborate_maybe_template_params(
    params: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<Vec<FormalSyntaxCatId>> {
    let params = reduce_to_builtin(params, &progress.macros, diags)?;

    // maybe_template_params = <empty> | "(" template_params ")"
    if let Some([]) = params.as_rule(*MAYBE_TEMPLATE_PARAMS_EMPTY_RULE) {
        Ok(Vec::new())
    } else if let Some([left_paren, template_params, right_paren]) =
        params.as_rule(*MAYBE_TEMPLATE_PARAMS_PARAMS_RULE)
    {
        let _ = assert!(left_paren.is_lit(*strings::LEFT_PAREN));
        let _ = assert!(right_paren.is_lit(*strings::RIGHT_PAREN));
        elaborate_template_params(template_params.clone(), progress, diags)
    } else {
        panic!("Failed to match builtin rule.");
    }
}

category_id!(TEMPLATE_PARAMS_CAT = "template_params");
rule_id!(TEMPLATE_PARAMS_ONE_RULE = "template_params_one");
rule_id!(TEMPLATE_PARAMS_MORE_RULE = "template_params_more");

fn elaborate_template_params(
    params: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<Vec<FormalSyntaxCatId>> {
    // template_params ::= template_param | template_param "," template_params

    let mut result = Vec::new();
    let mut next_params = Some(params);

    while let Some(params) = next_params {
        let builtin_params = reduce_to_builtin(params, &progress.macros, diags)?;

        if let Some([param]) = builtin_params.as_rule(*TEMPLATE_PARAMS_ONE_RULE) {
            next_params = None;
            let cat_id = elaborate_template_param(param.clone(), progress, diags)?;
            result.push(cat_id);
        } else if let Some([param, comma, rest]) =
            builtin_params.as_rule(*TEMPLATE_PARAMS_MORE_RULE)
        {
            assert!(comma.is_lit(*strings::COMMA));
            next_params = Some(rest.clone());
            let cat_id = elaborate_template_param(param.clone(), progress, diags)?;
            result.push(cat_id);
        } else {
            panic!("Failed to match builtin rule.");
        }
    }

    Ok(result)
}

category_id!(TEMPLATE_PARAM_CAT = "template_param");
rule_id!(TEMPLATE_PARAM_RULE = "template_param");

fn elaborate_template_param(
    param: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<FormalSyntaxCatId> {
    let param = reduce_to_builtin(param, &progress.macros, diags)?;

    // template_param ::= name ":" name
    let Some([name, colon, cat_name]) = param.as_rule(*TEMPLATE_PARAM_RULE) else {
        panic!("Failed to match template_param rule.");
    };

    let _ = assert!(colon.is_lit(*strings::COLON));
    let _name_str = name.as_name().unwrap();
    let cat_name_str = cat_name.as_name().unwrap();

    let cat_id = FormalSyntaxCatId::new(cat_name_str);
    if !progress.formal_syntax.has_cat(cat_id) {
        return diags.err_unknown_formal_syntax_cat();
    }

    Ok(cat_id)
}

category_id!(HYPOTHESES_CAT = "hypotheses");
rule_id!(HYPOTHESES_EMPTY_RULE = "hypotheses_empty");
rule_id!(HYPOTHESES_MORE_RULE = "hypotheses_more");

fn elaborate_hypotheses(
    mut hypotheses: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<Vec<UnresolvedFact>> {
    let mut facts = Vec::new();

    loop {
        let hypotheses_builtin = reduce_to_builtin(hypotheses, &progress.macros, diags)?;

        // hypotheses ::= <empty> | hypothesis hypotheses
        if let Some([]) = hypotheses_builtin.as_rule(*HYPOTHESES_EMPTY_RULE) {
            break;
        } else if let Some([hypothesis, rest]) = hypotheses_builtin.as_rule(*HYPOTHESES_MORE_RULE) {
            let fact = elaborate_hypothesis(hypothesis.clone(), progress, diags)?;
            facts.push(fact);
            hypotheses = rest.clone();
        } else {
            panic!("Failed to match builtin rule.");
        }
    }

    Ok(facts)
}

category_id!(HYPOTHESIS_CAT = "hypothesis");
rule_id!(HYPOTHESIS_RULE = "hypothesis");

fn elaborate_hypothesis(
    hypothesis: ParseTree,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFact> {
    let hypothesis = reduce_to_builtin(hypothesis, &progress.macros, diags)?;

    // hypothesis ::= "(" fact ")"
    let Some([left_paren, fact, right_paren]) = hypothesis.as_rule(*HYPOTHESIS_RULE) else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(left_paren.is_lit(*strings::LEFT_PAREN));
    let fact = elaborate_fact(
        fact.clone(),
        &progress.formal_syntax,
        &progress.macros,
        diags,
    )?;
    let _ = assert!(right_paren.is_lit(*strings::RIGHT_PAREN));

    Ok(fact)
}

category_id!(FACT_CAT = "fact");
rule_id!(FACT_SENTENCE_RULE = "fact_sentence");
rule_id!(FACT_ASSUME_RULE = "fact_assume");

pub fn elaborate_fact(
    fact: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFact> {
    let fact = reduce_to_builtin(fact, macros, diags)?;

    // fact ::= sentence | "assume" sentence "|-" sentence
    if let Some([sentence]) = fact.as_rule(*FACT_SENTENCE_RULE) {
        let sentence = elaborate_formal_fragment(sentence.clone(), formal_syntax, macros, diags)?;
        Ok(UnresolvedFact {
            assumption: None,
            statement: sentence,
        })
    } else if let Some([assume_kw, sentence, turnstile, statement]) =
        fact.as_rule(*FACT_ASSUME_RULE)
    {
        let _ = assert!(assume_kw.is_kw(*strings::ASSUME));
        let sentence = elaborate_formal_fragment(sentence.clone(), formal_syntax, macros, diags)?;
        let _ = assert!(turnstile.is_lit(*strings::TURNSTILE));
        let statement = elaborate_formal_fragment(statement.clone(), formal_syntax, macros, diags)?;
        return Ok(UnresolvedFact {
            assumption: Some(sentence),
            statement,
        });
    } else {
        panic!("Failed to match builtin rule.");
    }
}

fn elaborate_formal_fragment(
    frag: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFragment> {
    let og_span = frag.span();
    let og_rule = frag.as_node().unwrap().rule;
    let frag = reduce_to_builtin(frag, macros, diags)?;

    if let ParseTree::Node(node) = &frag
        && let ParseRuleId::FormalLang(_) = node.rule
    {
        elaborate_formal_rule(frag, og_span, og_rule, macros, formal_syntax, diags)
    } else if let ParseTree::Node(node) = &frag
        && let ParseRuleId::Pattern(kind, _) = node.rule
        && kind == *strings::FORMAL_TEMPLATE_RULE
    {
        elaborate_formal_template(frag, formal_syntax, macros, diags)
    } else {
        panic!("failed to match builtin rule");
    }
}

fn elaborate_formal_rule(
    frag: ParseTree,
    og_span: Span,
    og_rule: ParseRuleId,
    macros: &Macros,
    formal_syntax: &FormalSyntax,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFragment> {
    let node = frag.as_node().unwrap();
    let ParseNode {
        category: SyntaxCategoryId::FormalLang(formal_cat),
        rule: ParseRuleId::FormalLang(formal_rule),
        ..
    } = node
    else {
        panic!("Failed to match builtin rule");
    };

    let formal_pattern = formal_syntax.get_rule(*formal_rule).pat();
    let mut children = Vec::new();
    for (child, pat) in node.children.iter().zip(formal_pattern.parts()) {
        let child_frag =
            match pat {
                FormalSyntaxPatternPart::Cat(_) => UnresolvedFragPart::Frag(
                    elaborate_formal_fragment(child.clone(), formal_syntax, macros, diags)?,
                ),
                FormalSyntaxPatternPart::Lit(expected) => elaborate_formal_lit(child, *expected),
                FormalSyntaxPatternPart::Binding(cat) => elaborate_formal_binding(child, *cat),
                FormalSyntaxPatternPart::Variable(cat) => {
                    UnresolvedFragPart::Frag(elaborate_formal_var(child, *cat))
                }
            };
        children.push(child_frag);
    }

    Ok(UnresolvedFragment {
        _span: og_span,
        formal_cat: *formal_cat,
        data: UnresolvedFragmentData::FormalRule {
            _syntax_rule: og_rule,
            formal_rule: *formal_rule,
            children,
        },
    })
}

fn elaborate_formal_lit(lit: &ParseTree, expected: Ustr) -> UnresolvedFragPart {
    let atom = lit.as_atom().unwrap();
    let ParseAtomKind::Lit(got) = atom.kind else {
        panic!("failed to match builtin rule");
    };

    assert!(got == expected);
    UnresolvedFragPart::Lit
}

fn elaborate_formal_binding(binding: &ParseTree, cat: FormalSyntaxCatId) -> UnresolvedFragPart {
    let name = binding.as_name().unwrap();
    UnresolvedFragPart::Binding { name, cat }
}

fn elaborate_formal_var(var: &ParseTree, formal_cat: FormalSyntaxCatId) -> UnresolvedFragment {
    let name = var.as_name().unwrap();
    UnresolvedFragment {
        _span: var.span(),
        formal_cat,
        data: UnresolvedFragmentData::VarOrTemplate {
            name,
            args: Vec::new(),
        },
    }
}

fn elaborate_formal_template(
    template: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFragment> {
    // <formal_cat> ::= name maybe_template_args

    let node = template.as_node().unwrap();
    let ParseRuleId::Pattern(_, SyntaxCategoryId::FormalLang(formal_cat)) = node.rule else {
        panic!("Failed to match builtin rule.");
    };

    let Some([name, args]) = template.as_rule_pat(*strings::FORMAL_TEMPLATE_RULE) else {
        panic!("Failed to match builtin rule.");
    };

    let name_str = name.as_name().unwrap();
    let args = elaborate_maybe_template_args(args.clone(), formal_syntax, macros, diags)?;

    Ok(UnresolvedFragment {
        _span: template.span(),
        formal_cat,
        data: UnresolvedFragmentData::VarOrTemplate {
            name: name_str,
            args,
        },
    })
}

category_id!(MAYBE_TEMPLATE_ARGS_CAT = "maybe_template_args");
rule_id!(MAYBE_TEMPLATE_ARGS_EMPTY_RULE = "maybe_template_args_empty");
rule_id!(MAYBE_TEMPLATE_ARGS_ARGS_RULE = "maybe_template_args_args");

fn elaborate_maybe_template_args(
    args: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<Vec<UnresolvedFragment>> {
    // maybe_template_args = <empty> | "(" template_args ")"
    let args = reduce_to_builtin(args, macros, diags)?;

    if let Some([]) = args.as_rule(*MAYBE_TEMPLATE_ARGS_EMPTY_RULE) {
        Ok(Vec::new())
    } else if let Some([l_paren, args, r_paren]) = args.as_rule(*MAYBE_TEMPLATE_ARGS_ARGS_RULE) {
        let _ = assert!(l_paren.is_lit(*strings::LEFT_PAREN));
        let _ = assert!(r_paren.is_lit(*strings::RIGHT_PAREN));
        elaborate_template_args(args.clone(), formal_syntax, macros, diags)
    } else {
        panic!("Failed to match builtin rule.");
    }
}

category_id!(TEMPLATE_ARGS_CAT = "template_args");
rule_id!(TEMPLATE_ARGS_ONE_RULE = "template_args_one");
rule_id!(TEMPLATE_ARGS_MORE_RULE = "template_args_more");

fn elaborate_template_args(
    mut args: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<Vec<UnresolvedFragment>> {
    // template_args ::= template_arg | template_arg "," template_args

    let mut result = Vec::new();

    loop {
        let args_builtin = reduce_to_builtin(args, macros, diags)?;

        if let Some([arg]) = args_builtin.as_rule(*TEMPLATE_ARGS_ONE_RULE) {
            result.push(elaborate_template_arg(
                arg.clone(),
                formal_syntax,
                macros,
                diags,
            )?);
            break;
        } else if let Some([arg, comma, more]) = args_builtin.as_rule(*TEMPLATE_ARGS_MORE_RULE) {
            let _ = assert!(comma.is_lit(*strings::COMMA));
            result.push(elaborate_template_arg(
                arg.clone(),
                formal_syntax,
                macros,
                diags,
            )?);
            args = more.clone();
        } else {
            panic!("Failed to match builtin rule.");
        }
    }

    Ok(result)
}

category_id!(TEMPLATE_ARG_CAT = "template_arg");

fn elaborate_template_arg(
    arg: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFragment> {
    // template_arg ::= <formal_cat> ":" "formal_cat_name"

    let arg = reduce_to_builtin(arg, macros, diags)?;

    if let Some([arg, colon, _cat]) = arg.as_rule_pat(*strings::FORMAL_TEMPLATE_ARG_RULE) {
        let _ = assert!(colon.is_lit(*strings::COLON));
        elaborate_formal_fragment(arg.clone(), formal_syntax, macros, diags)
    } else {
        panic!("Failed to match builtin rule.")
    }
}

rule_id!(THEOREM_RULE = "theorem");

fn elaborate_theorem(
    theorem: ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<()> {
    let theorem = reduce_to_builtin(theorem, &progress.macros, diags)?;

    // command ::= "theorem" name templates ":" hypotheses "|-" sentence "proof" tactics "qed"
    let Some(
        [
            theorem_kw,
            name,
            templates,
            colon,
            hypotheses,
            turnstile,
            conclusion,
            proof_kw,
            tactics,
            qed_kw,
        ],
    ) = theorem.as_rule(*THEOREM_RULE)
    else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(theorem_kw.is_kw(*strings::THEOREM));
    let name_str = name.as_name().unwrap();
    let templates = elaborate_templates(templates.clone(), progress, diags)?;
    let _ = assert!(colon.is_lit(*strings::COLON));
    let hypotheses = elaborate_hypotheses(hypotheses.clone(), progress, diags)?;
    let _ = assert!(turnstile.is_lit(*strings::TURNSTILE));
    let conclusion = elaborate_formal_fragment(
        conclusion.clone(),
        &progress.formal_syntax,
        &progress.macros,
        diags,
    )?;
    let _ = assert!(proof_kw.is_kw(*strings::PROOF));
    let _ = assert!(qed_kw.is_kw(*strings::QED));

    let id = TheoremId::new(name_str);

    if progress.theorems.contains_key(&id) {
        return diags.err_duplicate_theorem(name_str, name.span());
    }

    let axiom = UnresolvedTheorem::new(
        id,
        templates,
        hypotheses,
        conclusion,
        UnresolvedProof::Theorem(tactics.clone()),
    );
    progress.theorems.insert(id, axiom);

    Ok(())
}

// tactics ::= <empty> | tactic tactics
// tactic ::= "by" name tactic_templates
//          | "have" fact tactics ";"
//          | "todo"

category_id!(TACTICS_CAT = "tactics");
rule_id!(TACTICS_EMPTY_RULE = "tactics_empty");
rule_id!(TACTICS_BY_RULE = "tactics_by");
rule_id!(TACTICS_HAVE_RULE = "tactics_have");
rule_id!(TACTICS_TODO_RULE = "tactics_todo");

category_id!(TACTIC_TEMPLATES_CAT = "tactic_templates");
rule_id!(TACTIC_TEMPLATES_EMPTY_RULE = "tactic_templates_empty");
rule_id!(TACTIC_TEMPLATES_MORE_RULE = "tactic_templates_more");

pub fn elaborate_tactic_templates(
    mut tactic_templates: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<Vec<UnresolvedFragment>> {
    let mut result = Vec::new();

    loop {
        let builtin = reduce_to_builtin(tactic_templates, macros, diags)?;

        if let Some([]) = builtin.as_rule(*TACTIC_TEMPLATES_EMPTY_RULE) {
            break;
        } else if let Some([template, rest]) = builtin.as_rule(*TACTIC_TEMPLATES_MORE_RULE) {
            let template =
                elaborate_tactic_template(template.clone(), formal_syntax, macros, diags)?;
            result.push(template);
            tactic_templates = rest.clone();
        } else {
            panic!("Failed to match builtin rule.");
        }
    }

    Ok(result)
}

category_id!(TACTIC_TEMPLATE_CAT = "tactic_template");

fn elaborate_tactic_template(
    tactic_template: ParseTree,
    formal_syntax: &FormalSyntax,
    macros: &Macros,
    diags: &mut DiagManager,
) -> WResult<UnresolvedFragment> {
    // tactic_template ::= "[" <formal_cat> ":" "formal_cat_name" "]"

    let tactic_template = reduce_to_builtin(tactic_template, macros, diags)?;
    if let Some([left_bracket, arg, colon, _cat, right_bracket]) =
        tactic_template.as_rule_pat(*strings::TACTIC_TEMPLATE_RULE)
    {
        let _ = assert!(left_bracket.is_lit(*strings::LEFT_BRACKET));
        let _ = assert!(colon.is_lit(*strings::COLON));
        let _ = assert!(right_bracket.is_lit(*strings::RIGHT_BRACKET));
        elaborate_formal_fragment(arg.clone(), formal_syntax, macros, diags)
    } else {
        panic!("Failed to match builtin rule.")
    }
}

pub(super) fn add_builtin_syntax(progress: &mut SourceParseProgress) {
    let mut insert = |cat, rule, pattern| {
        progress.add_rule(ParseRule {
            id: rule,
            cat,
            pattern,
        })
    };

    use AtomPattern as AP;
    use PatternPart as PP;

    let cat = |c| PP::Category(c);
    let kw = |s| PP::Atom(AP::Kw(s));
    let lit = |s| PP::Atom(AP::Lit(s));
    let name = || PP::Atom(AP::Name);
    let str = || PP::Atom(AP::Str);

    // command ::= "module" name
    //           | "syntax_cat" name
    //           | "syntax" name $category:name "::=" syntax_pat "end"

    insert(
        *COMMAND_CAT,
        *MODULE_RULE,
        vec![kw(*strings::MODULE), name()],
    );

    insert(
        *COMMAND_CAT,
        *SYNTAX_CAT_RULE,
        vec![kw(*strings::SYNTAX_CAT), name()],
    );

    insert(
        *COMMAND_CAT,
        *SYNTAX_RULE,
        vec![
            kw(*strings::SYNTAX),
            name(),
            name(),
            lit(*strings::BNF_REPLACE),
            cat(*SYNTAX_PAT_LIST),
            kw(*strings::END),
        ],
    );

    // syntax_pat_list ::= syntax_pat
    //                   | syntax_pat syntax_pat_list

    insert(
        *SYNTAX_PAT_LIST,
        *SYNTAX_PAT_LIST_ONE,
        vec![cat(*SYNTAX_PAT)],
    );
    insert(
        *SYNTAX_PAT_LIST,
        *SYNTAX_PAT_LIST_MORE,
        vec![cat(*SYNTAX_PAT), cat(*SYNTAX_PAT_LIST)],
    );

    // syntax_pat ::= name
    //             | "@" "binding" "(" name ")"
    //             | "@" "variable" "(" name ")"
    //             | str

    insert(*SYNTAX_PAT, *SYNTAX_PAT_NAME, vec![name()]);
    insert(
        *SYNTAX_PAT,
        *SYNTAX_PAT_BINDING,
        vec![
            lit(*strings::AT),
            kw(*strings::BINDING),
            lit(*strings::LEFT_PAREN),
            name(),
            lit(*strings::RIGHT_PAREN),
        ],
    );
    insert(
        *SYNTAX_PAT,
        *SYNTAX_PAT_VARIABLE,
        vec![
            lit(*strings::AT),
            kw(*strings::VARIABLE),
            lit(*strings::LEFT_PAREN),
            name(),
            lit(*strings::RIGHT_PAREN),
        ],
    );
    insert(*SYNTAX_PAT, *SYNTAX_PAT_STR, vec![str()]);

    // macro_pat_list ::= macro_pat
    //                  | macro_pat macro_pat_list

    insert(
        *MACRO_PAT_LIST_CAT,
        *MACRO_PAT_LIST_ONE,
        vec![cat(*MACRO_PAT_PART_CAT)],
    );
    insert(
        *MACRO_PAT_LIST_CAT,
        *MACRO_PAT_LIST_MORE,
        vec![cat(*MACRO_PAT_PART_CAT), cat(*MACRO_PAT_LIST_CAT)],
    );

    // macro_pat_part ::= macro_pat_binding macro_pat
    // macro_pat_binding ::= <nothing> | "$" name ":"
    // macro_pat ::= "@" "kw" str
    //             | "@" "name"
    //             | str
    //             | name
    //             | "@" "template" "(" name ")"
    insert(
        *MACRO_PAT_PART_CAT,
        *MACRO_PAT_PART_RULE,
        vec![cat(*MACRO_PAT_BINDING_CAT), cat(*MACRO_PAT_CAT)],
    );
    insert(*MACRO_PAT_BINDING_CAT, *MACRO_PAT_BINDING_NONE, vec![]);
    insert(
        *MACRO_PAT_BINDING_CAT,
        *MACRO_PAT_BINDING_NAME,
        vec![lit(*strings::DOLLAR), name(), lit(*strings::COLON)],
    );
    insert(
        *MACRO_PAT_CAT,
        *MACRO_PAT_KW,
        vec![lit(*strings::AT), kw(*strings::KW), str()],
    );
    insert(
        *MACRO_PAT_CAT,
        *MACRO_PAT_NAME,
        vec![lit(*strings::AT), kw(*strings::NAME)],
    );
    insert(*MACRO_PAT_CAT, *MACRO_PAT_STR, vec![str()]);
    insert(*MACRO_PAT_CAT, *MACRO_PAT_CAT_REF, vec![name()]);
    insert(
        *MACRO_PAT_CAT,
        *MACRO_PAT_TEMP_CAT_REF,
        vec![
            lit(*strings::AT),
            kw(*strings::TEMPLATE),
            lit(*strings::LEFT_PAREN),
            name(),
            lit(*strings::RIGHT_PAREN),
        ],
    );

    // command ::= "axiom" name templates ":" hypotheses "|-" sentence "end"
    insert(
        *COMMAND_CAT,
        *AXIOM_RULE,
        vec![
            kw(*strings::AXIOM),
            name(),
            cat(*TEMPLATES_CAT),
            lit(*strings::COLON),
            cat(*HYPOTHESES_CAT),
            lit(*strings::TURNSTILE),
            cat(SyntaxCategoryId::FormalLang(FormalSyntaxCatId::sentence())),
            kw(*strings::END),
        ],
    );

    // templates ::= <empty> | template templates
    // template ::= "[" name maybe_template_args ":" name "]"
    insert(*TEMPLATES_CAT, *TEMPLATES_EMPTY_RULE, vec![]);
    insert(
        *TEMPLATES_CAT,
        *TEMPLATES_MORE_RULE,
        vec![cat(*TEMPLATE_CAT), cat(*TEMPLATES_CAT)],
    );

    insert(
        *TEMPLATE_CAT,
        *TEMPLATE_RULE,
        vec![
            lit(*strings::LEFT_BRACKET),
            name(),
            cat(*MAYBE_TEMPLATE_PARAMS_CAT),
            lit(*strings::COLON),
            name(),
            lit(*strings::RIGHT_BRACKET),
        ],
    );

    // maybe_template_args = <empty> | "(" template_args ")"
    // template_args ::= template_arg | template_arg "," template_args
    // template_arg ::= name ":" name
    insert(
        *MAYBE_TEMPLATE_PARAMS_CAT,
        *MAYBE_TEMPLATE_PARAMS_EMPTY_RULE,
        vec![],
    );
    insert(
        *MAYBE_TEMPLATE_PARAMS_CAT,
        *MAYBE_TEMPLATE_PARAMS_PARAMS_RULE,
        vec![
            lit(*strings::LEFT_PAREN),
            cat(*TEMPLATE_PARAMS_CAT),
            lit(*strings::RIGHT_PAREN),
        ],
    );

    insert(
        *TEMPLATE_PARAMS_CAT,
        *TEMPLATE_PARAMS_ONE_RULE,
        vec![cat(*TEMPLATE_PARAM_CAT)],
    );
    insert(
        *TEMPLATE_PARAMS_CAT,
        *TEMPLATE_PARAMS_MORE_RULE,
        vec![
            cat(*TEMPLATE_PARAM_CAT),
            lit(*strings::COMMA),
            cat(*TEMPLATE_PARAMS_CAT),
        ],
    );

    insert(
        *TEMPLATE_PARAM_CAT,
        *TEMPLATE_PARAM_RULE,
        vec![name(), lit(*strings::COLON), name()],
    );

    // hypotheses ::= <empty> | hypothesis hypotheses
    // hypothesis ::= "(" fact ")"
    // fact ::= sentence | "assume" sentence "|-" sentence
    insert(*HYPOTHESES_CAT, *HYPOTHESES_EMPTY_RULE, vec![]);
    insert(
        *HYPOTHESES_CAT,
        *HYPOTHESES_MORE_RULE,
        vec![cat(*HYPOTHESIS_CAT), cat(*HYPOTHESES_CAT)],
    );

    insert(
        *HYPOTHESIS_CAT,
        *HYPOTHESIS_RULE,
        vec![
            lit(*strings::LEFT_PAREN),
            cat(*FACT_CAT),
            lit(*strings::RIGHT_PAREN),
        ],
    );
    insert(
        *FACT_CAT,
        *FACT_SENTENCE_RULE,
        vec![cat(SyntaxCategoryId::FormalLang(
            FormalSyntaxCatId::sentence(),
        ))],
    );
    insert(
        *FACT_CAT,
        *FACT_ASSUME_RULE,
        vec![
            kw(*strings::ASSUME),
            cat(SyntaxCategoryId::FormalLang(FormalSyntaxCatId::sentence())),
            lit(*strings::TURNSTILE),
            cat(SyntaxCategoryId::FormalLang(FormalSyntaxCatId::sentence())),
        ],
    );

    // <formal_cat> ::= name maybe_template_args
    // maybe_template_args = <empty> | "(" template_args ")"
    // template_args ::= template_arg | template_arg "," template_args
    // template_arg ::= <formal_cat> ":" "formal_cat_name"

    insert(
        *MAYBE_TEMPLATE_ARGS_CAT,
        *MAYBE_TEMPLATE_ARGS_EMPTY_RULE,
        vec![],
    );
    insert(
        *MAYBE_TEMPLATE_ARGS_CAT,
        *MAYBE_TEMPLATE_ARGS_ARGS_RULE,
        vec![
            lit(*strings::LEFT_PAREN),
            cat(*TEMPLATE_ARGS_CAT),
            lit(*strings::RIGHT_PAREN),
        ],
    );

    insert(
        *TEMPLATE_ARGS_CAT,
        *TEMPLATE_ARGS_ONE_RULE,
        vec![cat(*TEMPLATE_ARG_CAT)],
    );
    insert(
        *TEMPLATE_ARGS_CAT,
        *TEMPLATE_ARGS_MORE_RULE,
        vec![
            cat(*TEMPLATE_ARG_CAT),
            lit(*strings::COMMA),
            cat(*TEMPLATE_ARGS_CAT),
        ],
    );

    // command ::= "theorem" name templates ":" hypotheses "|-" sentence "proof" tactics "qed"
    insert(
        *COMMAND_CAT,
        *THEOREM_RULE,
        vec![
            kw(*strings::THEOREM),
            name(),
            cat(*TEMPLATES_CAT),
            lit(*strings::COLON),
            cat(*HYPOTHESES_CAT),
            lit(*strings::TURNSTILE),
            cat(SyntaxCategoryId::FormalLang(FormalSyntaxCatId::sentence())),
            kw(*strings::PROOF),
            cat(*TACTICS_CAT),
            kw(*strings::QED),
        ],
    );

    // tactics ::= <empty>
    //          | "by" name tactic_templates tactics
    //          | "have" fact tactics ";" tactics
    //          | "todo" tactics

    insert(*TACTICS_CAT, *TACTICS_EMPTY_RULE, vec![]);
    insert(
        *TACTICS_CAT,
        *TACTICS_BY_RULE,
        vec![
            kw(*strings::BY),
            name(),
            cat(*TACTIC_TEMPLATES_CAT),
            cat(*TACTICS_CAT),
        ],
    );
    insert(
        *TACTICS_CAT,
        *TACTICS_HAVE_RULE,
        vec![
            kw(*strings::HAVE),
            cat(*FACT_CAT),
            cat(*TACTICS_CAT),
            lit(*strings::SEMICOLON),
            cat(*TACTICS_CAT),
        ],
    );
    insert(
        *TACTICS_CAT,
        *TACTICS_TODO_RULE,
        vec![kw(*strings::TODO), cat(*TACTICS_CAT)],
    );

    // tactic_templates ::= <empty> | tactic_template tactic_templates
    insert(*TACTIC_TEMPLATES_CAT, *TACTIC_TEMPLATES_EMPTY_RULE, vec![]);
    insert(
        *TACTIC_TEMPLATES_CAT,
        *TACTIC_TEMPLATES_MORE_RULE,
        vec![cat(*TACTIC_TEMPLATE_CAT), cat(*TACTIC_TEMPLATES_CAT)],
    );
}

fn formal_syntax_rule_to_rule(rule: &FormalSyntaxRule) -> Option<ParseRule> {
    // If the rule allows a single variable, this could be could be confused
    // for a template. In that case we will handle it later but we want to be
    // sure we always parse it as a template.
    if let [FormalSyntaxPatternPart::Variable(_)] = rule.pat().parts() {
        return None;
    }

    use AtomPattern as AP;
    use FormalSyntaxPatternPart as FSPP;
    use PatternPart as PP;

    let cat = |c| PP::Category(c);
    let lit = |s| PP::Atom(AP::Lit(s));
    let name = || PP::Atom(AP::Name);

    let mut pattern = Vec::new();
    for part in rule.pat().parts() {
        let part = match part {
            FSPP::Cat(formal_cat) => cat(SyntaxCategoryId::FormalLang(*formal_cat)),
            FSPP::Lit(str) => lit(*str),
            FSPP::Binding(_) | FSPP::Variable(_) => name(),
        };
        pattern.push(part);
    }

    Some(ParseRule {
        id: ParseRuleId::FormalLang(rule.id()),
        cat: SyntaxCategoryId::FormalLang(rule.cat()),
        pattern,
    })
}

fn formal_syntax_cat_template(formal_cat: FormalSyntaxCatId, progress: &mut SourceParseProgress) {
    // for every formal syntax category we also allow names as templates and
    // arguments to those names.

    let mut insert = |cat, rule, pattern| {
        progress.add_rule(ParseRule {
            id: rule,
            cat,
            pattern,
        })
    };

    use AtomPattern as AP;
    use PatternPart as PP;

    let cat = |c| PP::Category(c);
    let kw = |s| PP::Atom(AP::Kw(s));
    let lit = |s| PP::Atom(AP::Lit(s));
    let name = || PP::Atom(AP::Name);

    // <formal_cat> ::= name maybe_template_args
    // maybe_template_args = <empty> | "(" template_args ")"
    // template_args ::= template_arg | template_arg "," template_args
    // template_arg ::= <cat> ":" "cat"

    let syntax_cat = SyntaxCategoryId::FormalLang(formal_cat);
    insert(
        syntax_cat,
        ParseRuleId::Pattern(*strings::FORMAL_TEMPLATE_RULE, syntax_cat),
        vec![name(), cat(*MAYBE_TEMPLATE_ARGS_CAT)],
    );

    insert(
        *TEMPLATE_ARG_CAT,
        ParseRuleId::Pattern(*strings::FORMAL_TEMPLATE_ARG_RULE, syntax_cat),
        vec![
            cat(SyntaxCategoryId::FormalLang(formal_cat)),
            lit(*strings::COLON),
            kw(formal_cat.name()),
        ],
    );

    insert(
        *TACTIC_TEMPLATE_CAT,
        ParseRuleId::Pattern(*strings::TACTIC_TEMPLATE_RULE, syntax_cat),
        vec![
            lit(*strings::LEFT_BRACKET),
            cat(SyntaxCategoryId::FormalLang(formal_cat)),
            lit(*strings::COLON),
            kw(formal_cat.name()),
            lit(*strings::RIGHT_BRACKET),
        ],
    );
}

pub(super) fn add_formal_lang_syntax(progress: &mut SourceParseProgress) {
    let rules: Vec<_> = progress
        .formal_syntax
        .rules()
        .flat_map(formal_syntax_rule_to_rule)
        .collect();

    for rule in rules {
        progress.add_rule(rule);
    }

    for cat in progress.formal_syntax.cats().cloned().collect_vec() {
        formal_syntax_cat_template(cat, progress);
    }
}

fn macro_to_rule(mac: &MacroInfo) -> ParseRule {
    let mut pattern = Vec::new();

    use AtomPattern as AP;
    use PatternPart as PP;

    for part in mac.pat().parts() {
        let part = match part {
            MacroPatPart::Cat(cat_id) => PP::Category(*cat_id),
            MacroPatPart::TempCat(cat_id) => PP::TemplateCat(*cat_id),
            MacroPatPart::Lit(lit_str) => PP::Atom(AP::Lit(*lit_str)),
            MacroPatPart::Kw(kw_str) => PP::Atom(AP::Kw(*kw_str)),
            MacroPatPart::Name => PP::Atom(AP::Name),
        };
        pattern.push(part);
    }

    ParseRule {
        id: ParseRuleId::Macro(mac.id()),
        cat: mac.cat(),
        pattern,
    }
}

pub(super) fn add_macro_syntax(progress: &mut SourceParseProgress) {
    let rules: Vec<_> = progress.macros.macros().map(macro_to_rule).collect();

    for rule in rules {
        progress.add_rule(rule);
    }
}

pub(super) fn add_macro_match_syntax(
    match_cat: SyntaxCategoryId,
    progress: &mut SourceParseProgress,
) {
    let mut insert = |cat, rule, pattern| {
        progress.add_rule(ParseRule {
            id: rule,
            cat,
            pattern,
        })
    };

    use AtomPattern as AP;
    use PatternPart as PP;

    let cat = |c| PP::Category(c);
    let kw = |s| PP::Atom(AP::Kw(s));
    let lit = |s| PP::Atom(AP::Lit(s));
    let name = || PP::Atom(AP::Name);

    // command ::= "macro" name $category:name "::=" macro_pat_list "=>" $category "end"

    insert(
        *COMMAND_CAT,
        ParseRuleId::Pattern(*strings::MACRO_RULE, match_cat),
        vec![
            kw(*strings::MACRO),
            name(),
            kw(match_cat.name()),
            lit(*strings::BNF_REPLACE),
            cat(*MACRO_PAT_LIST_CAT),
            lit(*strings::FAT_ARROW),
            PP::TemplateCat(match_cat),
            kw(*strings::END),
        ],
    );
}
