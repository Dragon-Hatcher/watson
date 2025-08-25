use crate::{
    diagnostics::DiagManager,
    parse::{
        SourceCache,
        builtin::{BuiltinCats, BuiltinRules},
        macros::Macros,
        parse_tree::ParseForest,
    },
    semant::formal_syntax::FormalSyntax,
};

pub struct Ctx {
    /// The syntax of the formal language. (Categories and rules.)
    pub formal_syntax: FormalSyntax,

    /// Macro definitions for use during parsing and elaboration.
    pub macros: Macros,

    /// The current state of the parser.
    pub parse_forest: ParseForest,

    /// Diagnostics manager for reporting errors and warnings.
    pub diags: DiagManager,

    /// Source code cache for storing and retrieving the text of source files.
    pub sources: SourceCache,

    pub builtin_cats: BuiltinCats,
    pub builtin_rules: BuiltinRules,
}

impl Ctx {
    pub fn new(source_cache: SourceCache) -> Self {
        let mut parse_forest = ParseForest::new();
        let builtin_cats = BuiltinCats::new(&mut parse_forest);
        let builtin_rules = BuiltinRules::new(&mut parse_forest, &builtin_cats);

        Ctx {
            formal_syntax: FormalSyntax::new(),
            macros: Macros::new(),
            parse_forest,
            diags: DiagManager::new(),
            sources: source_cache,
            builtin_cats,
            builtin_rules,
        }
    }
}
