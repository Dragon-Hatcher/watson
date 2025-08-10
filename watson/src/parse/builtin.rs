use crate::{
    category_id,
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceCache, SourceId, SourceParseProgress,
        elaborator::reduce_to_builtin,
        parse_tree::{
            AtomPattern, ParseRule, ParseRuleId, ParseTree, PatternPart, SyntaxCategoryId,
        },
    },
    rule_id,
    semant::formal_syntax::{
        FormalSyntaxCatId, FormalSyntaxPattern, FormalSyntaxPatternPart, FormalSyntaxRule,
        FormalSyntaxRuleId,
    },
    strings::{self, MACRO_RULE},
};
use std::fs;

category_id!(COMMAND_CAT = "command");

pub fn elaborate_command(
    command: ParseTree,
    progress: &mut SourceParseProgress,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) -> WResult<()> {
    let command = reduce_to_builtin(command, diags)?;

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
    // } else if command.as_rule(*MACRO_RULE).is_some() {
    // elaborate_macro();
    // Ok(())
    } else {
        dbg!(&command);
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
        let builtin_pat_list = reduce_to_builtin(pat_list, diags)?;

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

    let pat = reduce_to_builtin(pat.clone(), diags)?;

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

fn elaborate_macro() {
    todo!()
}

category_id!(MACRO_PAT_LIST_CAT = "macro_pat_list");
rule_id!(MACRO_PAT_LIST_ONE = "macro_pat_list_one");
rule_id!(MACRO_PAT_LIST_MORE = "macro_pat_list_more");

category_id!(MACRO_PAT_PART_CAT = "macro_pat_part");
rule_id!(MACRO_PAT_PART_RULE = "macro_pat_part");

category_id!(MACRO_PAT_BINDING_CAT = "macro_pat_binding");
rule_id!(MACRO_PAT_BINDING_NONE = "macro_pat_binding_none");
rule_id!(MACRO_PAT_BINDING_NAME = "macro_pat_binding_name");

category_id!(MACRO_PAT_CAT = "macro_pat");
rule_id!(MACRO_PAT_KW = "macro_pat_kw");
rule_id!(MACRO_PAT_NAME = "macro_pat_name");
rule_id!(MACRO_PAT_STR = "macro_pat_str");
rule_id!(MACRO_PAT_CAT_REF = "macro_pat_cat_ref");

fn elaborate_macro_pat() -> WResult<Vec<PatternPart>> {
    todo!()
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
    // macro_pat
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
