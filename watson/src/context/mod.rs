use rustc_hash::FxHashMap;

use crate::{
    config::WatsonConfig,
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
        presentation::{Pres, PresId},
        tactic::{
            syntax::{TacticCat, TacticCatId, TacticRule, TacticRuleId},
            tactic_manager::TacticManager,
        },
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

    /// Records information about tactics.
    pub tactic_manager: TacticManager<'ctx>,

    /// Diagnostics manager for reporting errors and warnings.
    pub diags: DiagManager<'ctx>,

    /// Source code cache for storing and retrieving the text of source files.
    pub sources: SourceCache,

    /// Project configuration.
    pub config: WatsonConfig,

    pub sentence_cat: FormalSyntaxCatId<'ctx>,
    pub tactic_cat: TacticCatId<'ctx>,
    pub builtin_cats: BuiltinCats<'ctx>,
    pub builtin_rules: BuiltinRules<'ctx>,
    pub single_name_notations: FxHashMap<FormalSyntaxCatId<'ctx>, NotationPatternId<'ctx>>,
}

impl<'ctx> Ctx<'ctx> {
    pub fn new(sources: SourceCache, config: WatsonConfig, arenas: &'ctx Arenas<'ctx>) -> Self {
        let mut parse_state = ParseState::new();
        let mut tactic_manager = TacticManager::new();

        let sentence_formal_cat = arenas
            .formal_cats
            .alloc(*strings::SENTENCE, FormalSyntaxCat::new(*strings::SENTENCE));

        let tactic_tactic_cat = arenas
            .tactic_cats
            .alloc(*strings::TACTIC, TacticCat::new(*strings::TACTIC));
        tactic_manager.use_tactic_cat(tactic_tactic_cat);

        // Create the tactic parse category before calling add_builtin_rules
        let tactic_parse_cat = crate::parse::parse_state::Category::new(
            tactic_tactic_cat.name(),
            crate::parse::parse_state::SyntaxCategorySource::Tactic(tactic_tactic_cat),
        );
        let tactic_parse_cat = arenas
            .parse_cats
            .alloc(tactic_tactic_cat.name(), tactic_parse_cat);
        parse_state.use_cat(tactic_parse_cat);

        let builtin_cats = BuiltinCats::new(arenas, &mut parse_state);
        let builtin_rules = add_builtin_rules(
            &arenas.parse_rules,
            &arenas.parse_cats,
            &mut parse_state,
            sentence_formal_cat,
            tactic_parse_cat,
            &builtin_cats,
        );

        let mut ctx = Self {
            arenas,
            scopes: ScopeArena::new(),
            parse_state,
            tactic_manager,
            diags: DiagManager::new(),
            sources,
            config,
            sentence_cat: sentence_formal_cat,
            tactic_cat: tactic_tactic_cat,
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
    pub tactic_cats: NamedArena<TacticCat, TacticCatId<'ctx>>,
    pub tactic_rules: NamedArena<TacticRule<'ctx>, TacticRuleId<'ctx>>,
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
            tactic_cats: NamedArena::new(),
            tactic_rules: NamedArena::new(),
            theorem_stmts: NamedArena::new(),
        }
    }
}
