use rustc_hash::FxHashMap;

use crate::{
    context::arena::{InternedArena, NamedArena, PlainArena, ScopeArena},
    diagnostics::DiagManager,
    parse::{
        SourceCache, add_formal_cat,
        grammar::{BuiltinCats, BuiltinRules, add_builtin_rules},
        parse_state::{Category, CategoryId, ParseState, Rule, RuleId},
        parse_tree::{ParseTree, ParseTreeId},
    },
    semant::{
        formal_syntax::{FormalSyntaxCat, FormalSyntaxCatId, FormalSyntaxRule, FormalSyntaxRuleId},
        fragment::{Fragment, FragmentId},
        notation::{NotationBinding, NotationBindingId, NotationPattern, NotationPatternId},
        presentation::{Pres, PresId, PresTree, PresTreeId},
        theorems::{TheoremId, TheoremStatement},
    },
    strings,
};

pub mod arena;

pub struct Ctx<'ctx> {
    pub arenas: &'ctx Arenas<'ctx>,
    pub scopes: ScopeArena<'ctx>,

    /// Information about how we should currently be parsing syntax.
    pub parse_state: ParseState<'ctx>,

    /// Diagnostics manager for reporting errors and warnings.
    pub diags: DiagManager<'ctx>,

    /// Source code cache for storing and retrieving the text of source files.
    pub sources: SourceCache,

    pub sentence_cat: FormalSyntaxCatId<'ctx>,
    pub builtin_cats: BuiltinCats<'ctx>,
    pub builtin_rules: BuiltinRules<'ctx>,
    pub single_name_notations: FxHashMap<FormalSyntaxCatId<'ctx>, NotationPatternId<'ctx>>,
}

impl<'ctx> Ctx<'ctx> {
    pub fn new(sources: SourceCache, arenas: &'ctx Arenas<'ctx>) -> Self {
        let mut parse_state = ParseState::new();

        let sentence_formal_cat = arenas
            .formal_cats
            .alloc(*strings::SENTENCE, FormalSyntaxCat::new(*strings::SENTENCE));

        let builtin_cats = BuiltinCats::new(arenas, &mut parse_state);
        let builtin_rules = add_builtin_rules(
            &arenas.parse_rules,
            &arenas.parse_cats,
            &mut parse_state,
            sentence_formal_cat,
            &builtin_cats,
        );

        let mut ctx = Self {
            arenas,
            scopes: ScopeArena::new(),
            parse_state,
            diags: DiagManager::new(),
            sources,
            sentence_cat: sentence_formal_cat,
            builtin_cats,
            builtin_rules,
            single_name_notations: FxHashMap::default(),
        };

        add_formal_cat(sentence_formal_cat, &mut ctx);

        ctx
    }
}

pub struct Arenas<'ctx> {
    pub parse_forest: InternedArena<ParseTree<'ctx>, ParseTreeId<'ctx>>,
    pub parse_cats: NamedArena<Category<'ctx>, CategoryId<'ctx>>,
    pub parse_rules: PlainArena<Rule<'ctx>, RuleId<'ctx>>,
    pub formal_cats: NamedArena<FormalSyntaxCat, FormalSyntaxCatId<'ctx>>,
    pub formal_rules: NamedArena<FormalSyntaxRule<'ctx>, FormalSyntaxRuleId<'ctx>>,
    pub notations: PlainArena<NotationPattern<'ctx>, NotationPatternId<'ctx>>,
    pub notation_bindings: InternedArena<NotationBinding<'ctx>, NotationBindingId<'ctx>>,
    pub fragments: InternedArena<Fragment<'ctx>, FragmentId<'ctx>>,
    pub presentations: InternedArena<Pres<'ctx>, PresId<'ctx>>,
    pub presentation_trees: InternedArena<PresTree<'ctx>, PresTreeId<'ctx>>,
    pub theorem_stmts: NamedArena<TheoremStatement<'ctx>, TheoremId<'ctx>>,
}

impl<'ctx> Arenas<'ctx> {
    pub fn new() -> Self {
        Self {
            parse_forest: InternedArena::new(),
            parse_cats: NamedArena::new(),
            parse_rules: PlainArena::new(),
            formal_cats: NamedArena::new(),
            formal_rules: NamedArena::new(),
            notations: PlainArena::new(),
            notation_bindings: InternedArena::new(),
            fragments: InternedArena::new(),
            presentations: InternedArena::new(),
            presentation_trees: InternedArena::new(),
            theorem_stmts: NamedArena::new(),
        }
    }
}
