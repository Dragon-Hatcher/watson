use crate::{
    category_id,
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceCache, SourceId, SourceParseProgress,
        earley::parse_category,
        elaborator::reduce_to_builtin,
        macros::{MacroId, MacroInfo, MacroPat},
        parse_tree::{
            AtomPattern, ParseAtomKind, ParseRule, ParseRuleId, ParseTree, PatternPart,
            SyntaxCategoryId,
        },
    },
    rule_id,
    semant::formal_syntax::{
        FormalSyntaxCatId, FormalSyntaxPattern, FormalSyntaxPatternPart, FormalSyntaxRule,
        FormalSyntaxRuleId,
    },
    strings::{self, MACRO_RULE},
};
use std::{collections::HashMap, fs};
use ustr::Ustr;

use super::macros::MacroPatPart;

category_id!(COMMAND_CAT = "command");

pub fn elaborate_command(
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
    } else {
        unreachable!("No elaborator for parse tree {:?}.", command);
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
    // syntax_pat ::= name | str

    let pat = reduce_to_builtin(pat.clone(), &progress.macros, diags)?;

    if let Some([at, name]) = pat.as_rule(*SYNTAX_PAT_VARIABLE) {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(name.is_kw(*strings::VARIABLE));

        Ok(FormalSyntaxPatternPart::Variable)
    } else if let Some([at, name]) = pat.as_rule(*SYNTAX_PAT_BINDING) {
        let _ = assert!(at.is_lit(*strings::AT));
        let _ = assert!(name.is_kw(*strings::BINDING));

        Ok(FormalSyntaxPatternPart::Binding)
    } else if let Some([name]) = pat.as_rule(*SYNTAX_PAT_NAME) {
        let name = name.as_name().unwrap();

        if name == *strings::BINDING {
            Ok(FormalSyntaxPatternPart::Binding)
        } else if name == *strings::VARIABLE {
            Ok(FormalSyntaxPatternPart::Variable)
        } else {
            let cat = FormalSyntaxCatId::new(name);
            if !progress.formal_syntax.has_cat(cat) {
                return diags.err_unknown_formal_syntax_cat();
            }

            Ok(FormalSyntaxPatternPart::Cat(cat))
        }
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

    let replacement = reparse_replacement(replacement, *cat, &pattern, sources, &progress, diags)?;

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
    cat: SyntaxCategoryId,
    pattern: &MacroPat,
    sources: &SourceCache,
    progress: &SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<ParseTree> {
    let span = replacement.span();
    let text = sources.get_text(span.source());
    let new = parse_category(
        text,
        span.start(),
        Some(span.end()),
        cat,
        &progress.rules,
        Some(pattern),
        diags,
    )
    .ok_or(())?;

    // TODO: There is no real justification for this but I'm too lazy right now.
    assert!(new.span() == span);
    Ok(new)
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
            todo!("Err: non-existent syntax category.");
        }
    } else {
        panic!("failed to match builtin rule")
    }
}

pub fn add_builtin_syntax(progress: &mut SourceParseProgress) {
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

    // syntax_pat ::= name | "@" name | str

    insert(*SYNTAX_PAT, *SYNTAX_PAT_NAME, vec![name()]);
    insert(
        *SYNTAX_PAT,
        *SYNTAX_PAT_BINDING,
        vec![lit(*strings::AT), kw(*strings::BINDING)],
    );
    insert(
        *SYNTAX_PAT,
        *SYNTAX_PAT_VARIABLE,
        vec![lit(*strings::AT), kw(*strings::VARIABLE)],
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
}

fn formal_syntax_rule_to_rule(rule: &FormalSyntaxRule) -> ParseRule {
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
            FSPP::Binding | FSPP::Variable => name(),
        };
        pattern.push(part);
    }

    ParseRule {
        id: ParseRuleId::FormalLang(rule.id()),
        cat: SyntaxCategoryId::FormalLang(rule.cat()),
        pattern,
    }
}

pub fn add_formal_lang_syntax(progress: &mut SourceParseProgress) {
    let rules: Vec<_> = progress
        .formal_syntax
        .rules()
        .map(formal_syntax_rule_to_rule)
        .collect();

    for rule in rules {
        progress.add_rule(rule);
    }
}

fn macro_to_rule(mac: &MacroInfo) -> ParseRule {
    let mut pattern = Vec::new();

    use AtomPattern as AP;
    use PatternPart as PP;

    let cat = |c| PP::Category(c);
    let kw = |s| PP::Atom(AP::Kw(s));
    let lit = |s| PP::Atom(AP::Lit(s));
    let name = || PP::Atom(AP::Name);

    for part in mac.pat().parts() {
        let part = match part {
            MacroPatPart::Cat(cat_id) => cat(*cat_id),
            MacroPatPart::Lit(lit_str) => lit(*lit_str),
            MacroPatPart::Kw(kw_str) => kw(*kw_str),
            MacroPatPart::Name => name(),
        };
        pattern.push(part);
    }

    ParseRule {
        id: ParseRuleId::Macro(mac.id()),
        cat: mac.cat(),
        pattern,
    }
}

pub fn add_macro_syntax(progress: &mut SourceParseProgress) {
    let rules: Vec<_> = progress.macros.macros().map(macro_to_rule).collect();

    for rule in rules {
        progress.add_rule(rule);
    }
}

pub fn add_macro_match_syntax(match_cat: SyntaxCategoryId, progress: &mut SourceParseProgress) {
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
            cat(match_cat),
            kw(*strings::END),
        ],
    );
}
