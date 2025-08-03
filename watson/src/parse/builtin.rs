use crate::{
    category_id,
    diagnostics::{DiagManager, WResult},
    parse::{
        SourceCache, SourceId, SourceParseProgress,
        elaborator::reduce_to_builtin,
        parse_tree::{AtomPattern, ParseRule, ParseRuleId, ParseTree, PatternPart},
    },
    rule_id, strings,
};
use std::{collections::HashMap, fs};

category_id!(COMMAND_CAT = "command");
rule_id!(MACRO_RULE = "macro");

pub fn elaborate_command(
    command: ParseTree,
    progress: &mut SourceParseProgress,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) -> WResult<ParseTree> {
    let command = reduce_to_builtin(command, diags)?;

    if command.as_rule(*MODULE_RULE).is_some() {
        let new_source = elaborate_module(&command, sources, diags)?;
        progress.next_sources.push_back(new_source);

        Ok(command)
    } else if command.as_rule(*SYNTAX_CAT_RULE).is_some() {
        elaborate_syntax_cat(&command, progress, diags)?;
        Ok(command)
    } else {
        unreachable!("No elaborator for parse tree.")
    }
}

rule_id!(MODULE_RULE = "module");

fn elaborate_module(
    module: &ParseTree,
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
        return diags.err_source_redeclaration();
    }

    let Ok(source_text) = fs::read_to_string(path) else {
        return diags.err_non_existent_file();
    };

    sources.add(source_id, source_text);

    Ok(source_id)
}

rule_id!(SYNTAX_CAT_RULE = "syntax_cat");

fn elaborate_syntax_cat(
    command: &ParseTree,
    progress: &mut SourceParseProgress,
    diags: &mut DiagManager,
) -> WResult<()> {
    // syntax_cat ::= "syntax_category" name

    let Some([syntax_cat_kw, name]) = command.as_rule(*SYNTAX_CAT_RULE) else {
        panic!("Failed to match builtin rule.");
    };

    let _ = assert!(syntax_cat_kw.is_kw(*strings::SYNTAX_CAT));
    let name_str = name.as_name().unwrap();

    if progress.formal_syntax_categories.contains(&name_str) {
        return diags.err_duplicate_formal_syntax_cat();
    }

    progress.formal_syntax_categories.insert(name_str);

    Ok(())
}

rule_id!(SYNTAX_RULE = "syntax");

category_id!(SYNTAX_PAT_LIST = "syntax_pat_list");
rule_id!(SYNTAX_PAT_LIST_ONE = "syntax_pat_list_one");
rule_id!(SYNTAX_PAT_LIST_MORE = "syntax_pat_list_more");

category_id!(SYNTAX_PAT = "syntax_pat");
rule_id!(SYNTAX_PAT_NAME = "syntax_pat_name");
rule_id!(SYNTAX_PAT_STR = "syntax_pat_str");

fn elaborate_syntax() {}

pub fn add_builtin_syntax(rules: &mut HashMap<ParseRuleId, ParseRule>) {
    let mut insert = |cat, rule, pattern| {
        rules.insert(
            rule,
            ParseRule {
                id: rule,
                cat,
                pattern,
            },
        )
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

    // syntax_pat_list ::= syntax_pat | syntax_pat_list syntax_pat
    insert(*SYNTAX_PAT_LIST, *SYNTAX_PAT_LIST_ONE, vec![cat(*SYNTAX_PAT)]);
    insert(*SYNTAX_PAT_LIST, *SYNTAX_PAT_LIST_MORE, vec![cat(*SYNTAX_PAT), cat(*SYNTAX_PAT_LIST)]);

    // syntax_pat ::= name | str
    insert(*SYNTAX_PAT, *SYNTAX_PAT_NAME, vec![name()]);
    insert(*SYNTAX_PAT, *SYNTAX_PAT_STR, vec![str()]);
}
