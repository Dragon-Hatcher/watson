use crate::{
    diagnostics::DiagManager,
    parse::{
        SourceCache,
        grammar::{
            BuiltinCats, BuiltinRules, add_builtin_rules, add_builtin_syntax_for_formal_cat,
        },
        macros::Macros,
        parse_state::ParseState,
        parse_tree::ParseForest,
    },
    semant::{
        formal_syntax::FormalSyntax, fragment::FragmentForest, proof_status::ProofStatuses,
        theorems::TheoremStatements,
    },
};

pub struct Ctx {
    /// Macro definitions for use during parsing and elaboration.
    pub macros: Macros,

    /// Stores all the parse trees created by parsing and macro expansion.
    pub parse_forest: ParseForest,

    /// Stores the current state of the parser.
    pub parse_state: ParseState,

    /// The syntax of the formal language. (Categories and rules.)
    pub formal_syntax: FormalSyntax,

    /// Fragments of sentences in the formal language.
    pub fragments: FragmentForest,

    /// All the existing theorems/axioms and what they state.
    pub theorem_stmts: TheoremStatements,

    /// The results of checking all proofs.
    pub proof_statuses: ProofStatuses,

    /// Diagnostics manager for reporting errors and warnings.
    pub diags: DiagManager,

    /// Source code cache for storing and retrieving the text of source files.
    pub sources: SourceCache,

    pub builtin_cats: BuiltinCats,
    pub builtin_rules: BuiltinRules,
}

impl Ctx {
    pub fn new(source_cache: SourceCache) -> Self {
        let formal_syntax = FormalSyntax::new();
        let mut parse_state = ParseState::new();
        let builtin_cats = BuiltinCats::new(&mut parse_state);
        let builtin_rules = add_builtin_rules(&mut parse_state, &formal_syntax, &builtin_cats);

        let mut ctx = Ctx {
            macros: Macros::new(),
            parse_forest: ParseForest::new(),
            parse_state,
            formal_syntax,
            fragments: FragmentForest::new(),
            theorem_stmts: TheoremStatements::new(),
            proof_statuses: ProofStatuses::new(),
            diags: DiagManager::new(),
            sources: source_cache,
            builtin_cats,
            builtin_rules,
        };

        add_builtin_syntax_for_formal_cat(ctx.formal_syntax.sentence_cat(), &mut ctx);

        ctx
    }
}
