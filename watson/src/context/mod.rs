use crate::{
    diagnostics::DiagManager,
    parse::{
        SourceCache,
        grammar::{
            BuiltinCats, BuiltinRules, add_builtin_rules, add_builtin_syntax_for_formal_cat,
        },
        macros::Macros,
        parse_state::{ParseRules, ParseState},
        parse_tree::ParseForest,
    },
    semant::{
        formal_syntax::{FormalSyntax, FormalSyntaxCat, FormalSyntaxCatId},
        fragment::FragmentForest,
        theorems::TheoremStatements,
    },
    strings,
};

pub mod arena;

pub struct Ctx<'ctx> {
    pub arenas: &'ctx Arenas<'ctx>,

    /// Information about how we should currently be parsing syntax.
    pub parse_state: ParseState<'ctx>,

    /// Diagnostics manager for reporting errors and warnings.
    pub diags: DiagManager,

    /// Source code cache for storing and retrieving the text of source files.
    pub sources: SourceCache,

    pub sentence_formal_cat: FormalSyntaxCatId<'ctx>,
    pub builtin_cats: BuiltinCats<'ctx>,
    pub builtin_rules: BuiltinRules<'ctx>,
}

impl<'ctx> Ctx<'ctx> {
    pub fn new(sources: SourceCache, arenas: &'ctx Arenas<'ctx>) -> Self {
        let mut parse_state = ParseState::new();

        let sentence_formal_cat = arenas
            .formal_syntax
            .add_cat(FormalSyntaxCat::new(*strings::SENTENCE));

        let builtin_cats = BuiltinCats::new(arenas, &mut parse_state);
        let builtin_rules = add_builtin_rules(
            &arenas.parse_rules,
            &mut parse_state,
            sentence_formal_cat,
            &builtin_cats,
        );

        let mut ctx = Self {
            arenas,
            parse_state,
            diags: DiagManager::new(),
            sources,
            sentence_formal_cat,
            builtin_cats,
            builtin_rules,
        };

        add_builtin_syntax_for_formal_cat(sentence_formal_cat, &mut ctx);

        ctx
    }
}

pub struct Arenas<'ctx> {
    /// Macro definitions for use during parsing and elaboration.
    pub macros: Macros<'ctx>,

    /// Stores all the parse trees created by parsing and macro expansion.
    pub parse_forest: ParseForest<'ctx>,

    /// Stores the current state of the parser.
    pub parse_rules: ParseRules<'ctx>,

    /// The syntax of the formal language. (Categories and rules.)
    pub formal_syntax: FormalSyntax<'ctx>,

    /// Fragments of sentences in the formal language.
    pub fragments: FragmentForest<'ctx>,

    /// All the existing theorems/axioms and what they state.
    pub theorem_stmts: TheoremStatements<'ctx>,
}

impl<'ctx> Arenas<'ctx> {
    pub fn new() -> Self {
        Self {
            macros: Macros::new(),
            parse_forest: ParseForest::new(),
            parse_rules: ParseRules::new(),
            formal_syntax: FormalSyntax::new(),
            fragments: FragmentForest::new(),
            theorem_stmts: TheoremStatements::new(),
        }
    }
}
