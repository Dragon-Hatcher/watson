use crate::{
    diagnostics::DiagManager,
    parse::{
        SourceCache,
        grammar::{BuiltinCats, BuiltinRules, add_builtin_rules},
        macros::Macros,
        parse_state::ParseState,
        parse_tree::ParseForest,
    },
    semant::formal_syntax::FormalSyntax,
};

pub struct Ctx {
    /// The syntax of the formal language. (Categories and rules.)
    pub formal_syntax: FormalSyntax,

    /// Macro definitions for use during parsing and elaboration.
    pub macros: Macros,

    /// Stores all the parse trees created by parsing and macro expansion.
    pub parse_forest: ParseForest,

    /// Stores the current state of the parser.
    pub parse_state: ParseState,

    /// Diagnostics manager for reporting errors and warnings.
    pub diags: DiagManager,

    /// Source code cache for storing and retrieving the text of source files.
    pub sources: SourceCache,

    pub builtin_cats: BuiltinCats,
    pub builtin_rules: BuiltinRules,
}

impl Ctx {
    pub fn new(source_cache: SourceCache) -> Self {
        let mut parse_state = ParseState::new();
        let builtin_cats = BuiltinCats::new(&mut parse_state);
        let builtin_rules = add_builtin_rules(&mut parse_state, &builtin_cats);

        Ctx {
            formal_syntax: FormalSyntax::new(),
            macros: Macros::new(),
            parse_forest: ParseForest::new(),
            parse_state,
            diags: DiagManager::new(),
            sources: source_cache,
            builtin_cats,
            builtin_rules,
        }
    }
}
