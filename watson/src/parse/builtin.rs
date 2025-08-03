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
    } else {
        unreachable!("No elaborator for parse tree.")
    }
}

rule_id!(MODULE_RULE = "module");

pub fn elaborate_module(
    module: &ParseTree,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
) -> WResult<SourceId> {
    // module ::= module path:name

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

    //
    // command ::= "module" name
    //

    insert(
        *COMMAND_CAT,
        *MODULE_RULE,
        vec![
            PatternPart::Atom(AtomPattern::Kw(*strings::MODULE)),
            PatternPart::Atom(AtomPattern::Name),
        ],
    );
}
