use crate::{
    context::{
        Ctx,
        arena::{NamedArena, PlainArena},
    },
    parse::parse_state::{
        Associativity, Category, CategoryId, ParseAtomPattern, ParseRuleSource, ParseState,
        Precedence, Rule, RuleId, RulePattern, RulePatternPart, SyntaxCategorySource,
    },
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
        fragment::{FragHead, FragRuleApplication, Fragment, FragmentId},
        notation::{
            NotationBinding, NotationBindingId, NotationPattern, NotationPatternId,
            NotationPatternPart,
        },
        presentation::{Pres, PresFrag, PresHead, PresTree, PresTreeId},
        scope::ScopeEntry,
    },
    strings,
};
use ustr::Ustr;

macro_rules! builtin_cats {
    ($struct_name:ident { $( $name:ident ),* $(,)? }) => {
        pub struct $struct_name<'ctx> {
            $( pub $name: $crate::parse::parse_state::CategoryId<'ctx>, )*
        }

        impl<'ctx> $struct_name<'ctx> {
            pub fn new(
                arenas: &'ctx $crate::context::Arenas<'ctx>,
                state: &mut $crate::parse::parse_state::ParseState<'ctx>,
            ) -> Self {
                Self {
                    $( $name: {
                        let name = ustr::Ustr::from(stringify!($name));
                        let cat = $crate::parse::parse_state::Category::new(name, $crate::parse::parse_state::SyntaxCategorySource::Builtin);
                        let cat = arenas.parse_cats.alloc(name, cat);
                        state.use_cat(cat);
                        cat
                    }, )*
                }
            }
        }
    };
}

macro_rules! builtin_rules {
    ($struct_name:ident { $( $name:ident ),* $(,)? }) => {
        pub struct $struct_name<'ctx> {
            $( pub $name: $crate::parse::parse_state::RuleId<'ctx>, )*
        }
    };
}

/*
Grammar of the Watson language:

command ::= (module_command)          module_command
          | (syntax_cat_command)       syntax_cat_command
          | (syntax_command)           syntax_command
          | (notation_command)         notation_command
          | (definition_command)       definition_command
          | (axiom_command)            axiom_command
          | (theorem_command)          theorem_command
          | (tactic_category_command)  tactic_category_command
          | (tactic_command)           tactic_command

module_command ::= (module) kw"module" name

syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name
syntax_command ::= (syntax) kw"syntax" name name prec_assoc "::=" syntax_pat kw"end"

tactic_category_command ::= (tactic_category) kw"tactic_category" name
tactic_command ::= (tactic) kw"tactic" name name "::=" tactic_pat kw"end"

prec_assoc ::= (prec_assoc_none)
             | (prec_assoc_some) "(" maybe_prec maybe_assoc ")"

maybe_prec ::= (prec_none)
             | (prec_some) number

maybe_assoc ::= (assoc_none)
              | (assoc_left)  "<"
              | (assoc_right) ">"

syntax_pat ::= (syntax_pat_one)  syntax_pat_part
             | (syntax_pat_many) syntax_pat_part syntax_pat

syntax_pat_part ::= (syntax_pat_cat)     name
                  | (syntax_pat_binding) "@" kw"binding" "(" name ")"
                  | (syntax_pat_lit)     str

notation_command ::= (notation) kw"notation" name name prec_assoc "::=" notation_pat kw"end"

notation_pat ::= (notation_pat_one)  notation_pat_part
               | (notation_pat_many) notation_pat_part notation_pat

notation_pat ::= (notation_pat_lit)     str
               | (notation_pat_kw)      "@" kw"kw" str
               | (notation_pat_name)    "@" kw"name"
               | (notation_pat_cat)     name
               | (notation_pat_binding) "@" kw"binding" "(" name ")"

tactic_pat ::= (tactic_pat_none)
             | (tactic_pat_many) tactic_pat_part tactic_pat

tactic_pat_part ::= (tactic_pat_part) maybe_label tactic_pat_part_core

maybe_label ::= (label_none)
              | (label_some) name ":"

tactic_pat_part_core ::= (core_lit)      str
                       | (core_kw)       "@" kw"kw" str
                       | (core_name)     "@" kw"name"
                       | (core_cat)      name
                       | (core_fragment) "@" kw"fragment"
                       | (core_fact)     "@" kw"fact"

definition_command ::= (definition) kw"definition" notation_binding ":=" any_fragment kw"end"

// notation_binding is created from each notation command

axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"
theorem_command ::= (theorem) kw"theorem" name templates ":" hypotheses "|-" sentence kw"proof" <tactic_cat> kw"qed"

templates ::= (template_none)
            | (template_many) template templates

template ::= (template) "[" template_bindings ":" name "]"

template_bindings ::= (template_bindings_none)
                    | (template_bindings_many) notation_binding template_bindings

hypotheses ::= (hypotheses_none)
             | (hypotheses_many) hypothesis hypotheses

hypothesis ::= (hypothesis) "(" fact ")"

fact ::= (fact_assumption) kw"assume" sentence "|-" sentence
       | (fact_sentence)   sentence

// tactic syntax is now user-defined via tactic_category and tactic commands

<formal_cat> ::= name maybe_shorthand_args

maybe_shorthand_args ::= (maybe_shorthand_args_none)
                       | (maybe_shorthand_args_some) "(" shorthand_args ")"

shorthand_args ::= (shorthand_args_one)  shorthand_arg
                 | (shorthand_args_many) shorthand_arg "," shorthand_args
shorthand_arg ::= (shorthand_arg) any_fragment

any_fragment ::= <formal_cat>
*/

builtin_cats! {
    BuiltinCats {
        command,
        module_command,
        syntax_cat_command,
        syntax_command,
        notation_command,
        definition_command,
        axiom_command,
        theorem_command,
        tactic_category_command,
        tactic_command,
        prec_assoc,
        maybe_prec,
        maybe_assoc,
        syntax_pat,
        syntax_pat_part,
        notation_pat,
        notation_pat_part,
        tactic_pat,
        tactic_pat_part,
        maybe_label,
        tactic_pat_part_core,
        notation_binding,
        templates,
        template,
        template_bindings,
        hypotheses,
        hypothesis,
        fact,
        tactic,
        template_instantiations,
        template_instantiation,
        maybe_shorthand_args,
        shorthand_args,
        shorthand_arg,
        any_fragment,
        name,
        str,
    }
}

builtin_rules! {
    BuiltinRules {
        name,
        str,
        module_command,
        syntax_cat_command,
        syntax_command,
        notation_command,
        definition_command,
        axiom_command,
        theorem_command,
        tactic_category_command,
        tactic_command,
        module,
        syntax_cat,
        syntax,
        tactic_category,
        tactic,
        prec_assoc_none,
        prec_assoc_some,
        prec_none,
        prec_some,
        assoc_none,
        assoc_left,
        assoc_right,
        syntax_pat_one,
        syntax_pat_many,
        syntax_pat_part_cat,
        syntax_pat_part_binding,
        syntax_pat_part_var,
        syntax_pat_part_lit,
        notation,
        notation_pat_one,
        notation_pat_many,
        notation_pat_lit,
        notation_pat_kw,
        notation_pat_cat,
        notation_pat_name,
        notation_pat_binding,
        tactic_pat_none,
        tactic_pat_many,
        tactic_pat_part,
        label_none,
        label_some,
        core_lit,
        core_kw,
        core_name,
        core_cat,
        core_fragment,
        core_fact,
        definition,
        theorem,
        axiom,
        template_none,
        template_many,
        template,
        template_bindings_none,
        template_bindings_many,
        hypotheses_none,
        hypotheses_many,
        hypothesis,
        fact_assumption,
        fact_sentence,
        template_instantiations_none,
        template_instantiations_many,
        template_instantiation,
        maybe_shorthand_args_none,
        maybe_shorthand_args_some,
        shorthand_args_one,
        shorthand_args_many,
        shorthand_arg,
    }
}

fn kw(kw: Ustr) -> RulePatternPart<'static> {
    RulePatternPart::Atom(ParseAtomPattern::Kw(kw))
}

fn lit(lit: Ustr) -> RulePatternPart<'static> {
    RulePatternPart::Atom(ParseAtomPattern::Lit(lit))
}

fn num() -> RulePatternPart<'static> {
    RulePatternPart::Atom(ParseAtomPattern::Num)
}

fn cat(cat: CategoryId) -> RulePatternPart {
    RulePatternPart::Cat(cat)
}

pub fn add_builtin_rules<'ctx>(
    rules: &'ctx PlainArena<Rule<'ctx>, RuleId<'ctx>>,
    categories: &'ctx NamedArena<Category<'ctx>, CategoryId<'ctx>>,
    state: &mut ParseState<'ctx>,
    formal_sentence_cat: FormalSyntaxCatId<'ctx>,
    cats: &BuiltinCats<'ctx>,
) -> BuiltinRules<'ctx> {
    let sentence_cat = Category::new(
        *strings::SENTENCE,
        SyntaxCategorySource::FormalLang(formal_sentence_cat),
    );
    let sentence_cat = categories.alloc(*strings::SENTENCE, sentence_cat);
    state.use_cat(sentence_cat);

    let mut rule = |name: &str, cat, parts| {
        let rule = rules.alloc(Rule::new(
            name,
            cat,
            ParseRuleSource::Builtin,
            RulePattern::new(parts, Precedence::default(), Associativity::default()),
        ));
        state.use_rule(rule);
        rule
    };

    let rules = BuiltinRules {
        name: rule(
            "name",
            cats.name,
            vec![RulePatternPart::Atom(ParseAtomPattern::Name)],
        ),
        str: rule(
            "str",
            cats.str,
            vec![RulePatternPart::Atom(ParseAtomPattern::Str)],
        ),
        module_command: rule(
            "module_command",
            cats.command,
            vec![cat(cats.module_command)],
        ),
        syntax_cat_command: rule(
            "syntax_cat_command",
            cats.command,
            vec![cat(cats.syntax_cat_command)],
        ),
        syntax_command: rule(
            "syntax_command",
            cats.command,
            vec![cat(cats.syntax_command)],
        ),
        notation_command: rule(
            "notation_command",
            cats.command,
            vec![cat(cats.notation_command)],
        ),
        definition_command: rule(
            "definition_command",
            cats.command,
            vec![cat(cats.definition_command)],
        ),
        axiom_command: rule("axiom_command", cats.command, vec![cat(cats.axiom_command)]),
        theorem_command: rule(
            "theorem_command",
            cats.command,
            vec![cat(cats.theorem_command)],
        ),
        tactic_category_command: rule(
            "tactic_category_command",
            cats.command,
            vec![cat(cats.tactic_category_command)],
        ),
        tactic_command: rule(
            "tactic_command",
            cats.command,
            vec![cat(cats.tactic_command)],
        ),
        module: rule(
            "module",
            cats.module_command,
            vec![kw(*strings::MODULE), cat(cats.name)],
        ),
        syntax_cat: rule(
            "syntax_cat",
            cats.syntax_cat_command,
            vec![kw(*strings::SYNTAX_CAT), cat(cats.name)],
        ),
        syntax: rule(
            "syntax",
            cats.syntax_command,
            vec![
                kw(*strings::SYNTAX),
                cat(cats.name),
                cat(cats.name),
                cat(cats.prec_assoc),
                lit(*strings::BNF_REPLACE),
                cat(cats.syntax_pat),
                kw(*strings::END),
            ],
        ),
        tactic_category: rule(
            "tactic_category",
            cats.tactic_category_command,
            vec![kw(*strings::TACTIC_CATEGORY), cat(cats.name)],
        ),
        tactic: rule(
            "tactic",
            cats.tactic_command,
            vec![
                kw(*strings::TACTIC),
                cat(cats.name),
                cat(cats.name),
                lit(*strings::BNF_REPLACE),
                cat(cats.tactic_pat),
                kw(*strings::END),
            ],
        ),
        notation: rule(
            "notation",
            cats.notation_command,
            vec![
                kw(*strings::NOTATION),
                cat(cats.name),
                cat(cats.name),
                cat(cats.prec_assoc),
                lit(*strings::BNF_REPLACE),
                cat(cats.notation_pat),
                kw(*strings::END),
            ],
        ),
        definition: rule(
            "definition",
            cats.definition_command,
            vec![
                kw(*strings::DEFINITION),
                cat(cats.notation_binding),
                lit(*strings::ASSIGN),
                cat(cats.any_fragment),
                kw(*strings::END),
            ],
        ),
        axiom: rule(
            "axiom",
            cats.axiom_command,
            vec![
                kw(*strings::AXIOM),
                cat(cats.name),
                cat(cats.templates),
                lit(*strings::COLON),
                cat(cats.hypotheses),
                lit(*strings::TURNSTILE),
                cat(sentence_cat),
                kw(*strings::END),
            ],
        ),
        theorem: rule(
            "theorem",
            cats.theorem_command,
            vec![
                kw(*strings::THEOREM),
                cat(cats.name),
                cat(cats.templates),
                lit(*strings::COLON),
                cat(cats.hypotheses),
                lit(*strings::TURNSTILE),
                cat(sentence_cat),
                kw(*strings::PROOF),
                cat(cats.tactic),
                kw(*strings::QED),
            ],
        ),

        prec_assoc_none: rule("prec_assoc_none", cats.prec_assoc, vec![]),
        prec_assoc_some: rule(
            "prec_assoc_some",
            cats.prec_assoc,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.maybe_prec),
                cat(cats.maybe_assoc),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        prec_none: rule("prec_none", cats.maybe_prec, vec![]),
        prec_some: rule("prec_some", cats.maybe_prec, vec![num()]),
        assoc_none: rule("assoc_none", cats.maybe_assoc, vec![]),
        assoc_left: rule(
            "assoc_left",
            cats.maybe_assoc,
            vec![lit(*strings::LEFT_ARROW)],
        ),
        assoc_right: rule(
            "assoc_left",
            cats.maybe_assoc,
            vec![lit(*strings::RIGHT_ARROW)],
        ),

        syntax_pat_one: rule(
            "syntax_pat_one",
            cats.syntax_pat,
            vec![cat(cats.syntax_pat_part)],
        ),
        syntax_pat_many: rule(
            "syntax_pat_many",
            cats.syntax_pat,
            vec![cat(cats.syntax_pat_part), cat(cats.syntax_pat)],
        ),

        syntax_pat_part_cat: rule(
            "syntax_pat_part_cat",
            cats.syntax_pat_part,
            vec![cat(cats.name)],
        ),
        syntax_pat_part_binding: rule(
            "syntax_pat_part_binding",
            cats.syntax_pat_part,
            vec![
                lit(*strings::AT),
                kw(*strings::BINDING),
                lit(*strings::LEFT_PAREN),
                cat(cats.name),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        syntax_pat_part_var: rule(
            "syntax_pat_part_var",
            cats.syntax_pat_part,
            vec![
                lit(*strings::AT),
                kw(*strings::VARIABLE),
                lit(*strings::LEFT_PAREN),
                cat(cats.name),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        syntax_pat_part_lit: rule(
            "syntax_pat_part_lit",
            cats.syntax_pat_part,
            vec![cat(cats.str)],
        ),

        notation_pat_one: rule(
            "notation_pat_one",
            cats.notation_pat,
            vec![cat(cats.notation_pat_part)],
        ),
        notation_pat_many: rule(
            "notation_pat_many",
            cats.notation_pat,
            vec![cat(cats.notation_pat_part), cat(cats.notation_pat)],
        ),
        notation_pat_lit: rule(
            "notation_pat_lit",
            cats.notation_pat_part,
            vec![cat(cats.str)],
        ),
        notation_pat_kw: rule(
            "notation_pat_kw",
            cats.notation_pat_part,
            vec![lit(*strings::AT), kw(*strings::KW), cat(cats.str)],
        ),
        notation_pat_name: rule(
            "notation_pat_name",
            cats.notation_pat_part,
            vec![lit(*strings::AT), kw(*strings::NAME)],
        ),
        notation_pat_cat: rule(
            "notation_pat_cat",
            cats.notation_pat_part,
            vec![cat(cats.name)],
        ),
        notation_pat_binding: rule(
            "notation_pat_binding",
            cats.notation_pat_part,
            vec![
                lit(*strings::AT),
                kw(*strings::BINDING),
                lit(*strings::LEFT_PAREN),
                cat(cats.name),
                lit(*strings::RIGHT_PAREN),
            ],
        ),

        tactic_pat_none: rule("tactic_pat_none", cats.tactic_pat, vec![]),
        tactic_pat_many: rule(
            "tactic_pat_many",
            cats.tactic_pat,
            vec![cat(cats.tactic_pat_part), cat(cats.tactic_pat)],
        ),
        tactic_pat_part: rule(
            "tactic_pat_part",
            cats.tactic_pat_part,
            vec![cat(cats.maybe_label), cat(cats.tactic_pat_part_core)],
        ),

        label_none: rule("label_none", cats.maybe_label, vec![]),
        label_some: rule(
            "label_some",
            cats.maybe_label,
            vec![cat(cats.name), lit(*strings::COLON)],
        ),

        core_lit: rule("core_lit", cats.tactic_pat_part_core, vec![cat(cats.str)]),
        core_kw: rule(
            "core_kw",
            cats.tactic_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::KW), cat(cats.str)],
        ),
        core_name: rule(
            "core_name",
            cats.tactic_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::NAME)],
        ),
        core_cat: rule("core_cat", cats.tactic_pat_part_core, vec![cat(cats.name)]),
        core_fragment: rule(
            "core_fragment",
            cats.tactic_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::FRAGMENT)],
        ),
        core_fact: rule(
            "core_fact",
            cats.tactic_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::FACT)],
        ),

        template_none: rule("template_none", cats.templates, vec![]),
        template_many: rule(
            "template_many",
            cats.templates,
            vec![cat(cats.template), cat(cats.templates)],
        ),

        template: rule(
            "template",
            cats.template,
            vec![
                lit(*strings::LEFT_BRACKET),
                cat(cats.template_bindings),
                lit(*strings::COLON),
                cat(cats.name),
                lit(*strings::RIGHT_BRACKET),
            ],
        ),

        template_bindings_none: rule("template_bindings_none", cats.template_bindings, vec![]),
        template_bindings_many: rule(
            "template_bindings_many",
            cats.template_bindings,
            vec![cat(cats.notation_binding), cat(cats.template_bindings)],
        ),

        hypotheses_none: rule("hypotheses_none", cats.hypotheses, vec![]),
        hypotheses_many: rule(
            "hypotheses_many",
            cats.hypotheses,
            vec![cat(cats.hypothesis), cat(cats.hypotheses)],
        ),

        hypothesis: rule(
            "hypothesis",
            cats.hypothesis,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.fact),
                lit(*strings::RIGHT_PAREN),
            ],
        ),

        fact_assumption: rule(
            "fact_assumption",
            cats.fact,
            vec![
                kw(*strings::ASSUME),
                cat(sentence_cat),
                lit(*strings::TURNSTILE),
                cat(sentence_cat),
            ],
        ),
        fact_sentence: rule("fact_sentence", cats.fact, vec![cat(sentence_cat)]),

        template_instantiations_none: rule(
            "template_instantiations_none",
            cats.template_instantiations,
            vec![],
        ),
        template_instantiations_many: rule(
            "template_instantiations_many",
            cats.template_instantiations,
            vec![
                cat(cats.template_instantiation),
                cat(cats.template_instantiations),
            ],
        ),
        template_instantiation: rule(
            "template_instantiation",
            cats.template_instantiation,
            vec![
                lit(*strings::LEFT_BRACKET),
                cat(cats.any_fragment),
                lit(*strings::RIGHT_BRACKET),
            ],
        ),
        maybe_shorthand_args_none: rule(
            "maybe_shorthand_args_none",
            cats.maybe_shorthand_args,
            vec![],
        ),
        maybe_shorthand_args_some: rule(
            "maybe_shorthand_args_some",
            cats.maybe_shorthand_args,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.shorthand_args),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        shorthand_args_one: rule(
            "shorthand_args_one",
            cats.shorthand_args,
            vec![cat(cats.shorthand_arg)],
        ),
        shorthand_args_many: rule(
            "shorthand_args_many",
            cats.shorthand_args,
            vec![
                cat(cats.shorthand_arg),
                lit(*strings::COMMA),
                cat(cats.shorthand_args),
            ],
        ),
        shorthand_arg: rule(
            "shorthand_arg",
            cats.shorthand_arg,
            vec![cat(cats.any_fragment)],
        ),
    };
    state.recompute_initial_atoms();
    rules
}

pub fn add_parse_rules_for_formal_cat<'ctx>(
    formal_cat: FormalSyntaxCatId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) {
    let parse_cat = ctx.parse_state.cat_for_formal_cat(formal_cat);

    let rule = ctx.arenas.parse_rules.alloc(Rule::new(
        "any_fragment",
        ctx.builtin_cats.any_fragment,
        ParseRuleSource::Builtin,
        RulePattern::new(
            vec![cat(parse_cat)],
            Precedence::default(),
            Associativity::default(),
        ),
    ));
    ctx.parse_state.use_rule(rule);
}

pub fn formal_rule_to_notation<'ctx>(
    rule: FormalSyntaxRuleId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> (
    NotationPatternId<'ctx>,
    NotationBindingId<'ctx>,
    ScopeEntry<'ctx>,
) {
    fn to_notation<'ctx>(
        rule: FormalSyntaxRuleId<'ctx>,
        ctx: &mut Ctx<'ctx>,
    ) -> NotationPatternId<'ctx> {
        let mut parts = Vec::new();

        for formal_part in rule.pattern().parts() {
            let part = match formal_part {
                FormalSyntaxPatPart::Cat(cat) => NotationPatternPart::Cat(*cat),
                FormalSyntaxPatPart::Binding(cat) => NotationPatternPart::Binding(*cat),
                FormalSyntaxPatPart::Lit(lit) => NotationPatternPart::Lit(*lit),
            };
            parts.push(part);
        }

        let pattern = NotationPattern::new(
            rule.name(),
            rule.cat(),
            parts,
            rule.pattern().precedence(),
            rule.pattern().associativity(),
        );
        ctx.arenas.notations.alloc(pattern)
    }

    fn to_frag<'ctx>(rule: FormalSyntaxRuleId<'ctx>, ctx: &mut Ctx<'ctx>) -> FragmentId<'ctx> {
        let mut frag_children = Vec::new();
        let mut bindings_added = 0;
        for formal_part in rule.pattern().parts() {
            match formal_part {
                FormalSyntaxPatPart::Cat(cat) => {
                    let frag = Fragment::new(*cat, FragHead::Hole(frag_children.len()), Vec::new());
                    let frag = ctx.arenas.fragments.intern(frag);
                    frag_children.push(frag);
                }
                FormalSyntaxPatPart::Binding(_) => bindings_added += 1,
                FormalSyntaxPatPart::Lit(_) => continue,
            };
        }

        let rule_app = FragRuleApplication::new(rule, bindings_added);
        let frag = Fragment::new(
            rule.cat(),
            FragHead::RuleApplication(rule_app),
            frag_children,
        );
        ctx.arenas.fragments.intern(frag)
    }

    fn to_pres<'ctx>(
        rule: FormalSyntaxRuleId<'ctx>,
        binding: NotationBindingId<'ctx>,
        ctx: &mut Ctx<'ctx>,
    ) -> PresTreeId<'ctx> {
        let mut children = Vec::new();
        let mut trees = Vec::new();

        for formal_part in rule.pattern().parts() {
            match formal_part {
                FormalSyntaxPatPart::Cat(_) => {
                    let pres = Pres::new(PresHead::Hole(children.len()), Vec::new());
                    let pres = ctx.arenas.presentations.intern(pres);
                    let tree = PresTree::new(pres, Vec::new());
                    let tree = ctx.arenas.presentation_trees.intern(tree);

                    children.push(pres);
                    trees.push(tree);
                }
                FormalSyntaxPatPart::Binding(_) => {
                    todo!("Handle bindings")
                }
                FormalSyntaxPatPart::Lit(_) => continue,
            };
        }

        let parent_pres = Pres::new(PresHead::Notation(binding), children);
        let parent_pres = ctx.arenas.presentations.intern(parent_pres);
        let parent_tree = PresTree::new(parent_pres, trees);
        ctx.arenas.presentation_trees.intern(parent_tree)
    }

    let pattern = to_notation(rule, ctx);

    let binding = NotationBinding::new(pattern, Vec::new());
    let binding = ctx.arenas.notation_bindings.intern(binding);

    let frag = to_frag(rule, ctx);
    let pres = to_pres(rule, binding, ctx);
    let scope_entry = ScopeEntry::new(PresFrag(frag, pres));

    (pattern, binding, scope_entry)
}

fn fragment_parse_rule_for_notation<'ctx>(
    notation: NotationPatternId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> RuleId<'ctx> {
    let mut parts = Vec::new();
    for &notation_part in notation.0.parts() {
        let part = match notation_part {
            NotationPatternPart::Lit(lit_str) => lit(lit_str),
            NotationPatternPart::Kw(kw_str) => kw(kw_str),
            NotationPatternPart::Name => cat(ctx.builtin_cats.name),
            NotationPatternPart::Cat(formal_cat) => {
                let cat = ctx.parse_state.cat_for_formal_cat(formal_cat);
                RulePatternPart::Cat(cat)
            }
            NotationPatternPart::Binding(_) => cat(ctx.builtin_cats.name),
        };
        parts.push(part);
    }

    let parse_pat = RulePattern::new(parts, notation.prec(), notation.assoc());

    let parse_rule = Rule::new(
        notation.name(),
        ctx.parse_state.cat_for_formal_cat(notation.cat()),
        ParseRuleSource::Notation(notation),
        parse_pat,
    );

    ctx.arenas.parse_rules.alloc(parse_rule)
}

fn binding_parse_rule_for_notation<'ctx>(
    notation: NotationPatternId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> RuleId<'ctx> {
    let mut parts = Vec::new();
    for notation_part in notation.0.parts() {
        let part = match notation_part {
            NotationPatternPart::Lit(lit_str) => lit(*lit_str),
            NotationPatternPart::Kw(kw_str) => kw(*kw_str),
            NotationPatternPart::Name
            | NotationPatternPart::Cat(_)
            | NotationPatternPart::Binding(_) => cat(ctx.builtin_cats.name),
        };
        parts.push(part)
    }

    let parse_pat = RulePattern::new(parts, Precedence::default(), Associativity::default());

    let parse_rule = Rule::new(
        "notation_binding",
        ctx.builtin_cats.notation_binding,
        ParseRuleSource::Notation(notation),
        parse_pat,
    );

    ctx.arenas.parse_rules.alloc(parse_rule)
}

pub fn add_parse_rules_for_notation<'ctx>(notation: NotationPatternId<'ctx>, ctx: &mut Ctx<'ctx>) {
    let fragment_rule = fragment_parse_rule_for_notation(notation, ctx);
    ctx.parse_state.use_rule(fragment_rule);

    let binding_rule = binding_parse_rule_for_notation(notation, ctx);
    ctx.parse_state.use_rule(binding_rule);
}
