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
        commands::CommandId,
        custom_grammar::syntax::{CustomGrammarPatPartCore, CustomGrammarRuleId},
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
        fragment::{FragHead, FragRuleApplication, Fragment, hole_frag, var_frag},
        notation::{
            NotationBinding, NotationBindingId, NotationPattern, NotationPatternId,
            NotationPatternPart, NotationPatternPartCat, NotationPatternSource,
        },
        presentation::{Pres, PresFrag, PresHead},
        scope::{DefinitionSource, ScopeEntry},
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

command_decl ::= (command_decl) maybe_attribute_anno command

command ::= (module_command)           module_command
          | (syntax_cat_command)       syntax_cat_command
          | (syntax_command)           syntax_command
          | (notation_command)         notation_command
          | (definition_command)       definition_command
          | (axiom_command)            axiom_command
          | (theorem_command)          theorem_command
          | (grammar_category_command) grammar_category_command
          | (tactic_command)           tactic_command
          | (attribute_command)        attribute_command

maybe_attribute_anno ::= (attribute_anno_some) attribute_anno
                       | (attribute_anno_none)

attribute_anno ::= (attribute_anno) "@" "[" attributes "]"

attributes ::= (attributes_one)  attribute
             | (attributes_many) attribute "," attributes

// attribute filled in dynamically

module_command ::= (module) kw"module" name

syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name
syntax_command ::= (syntax) kw"syntax" name name prec_assoc "::=" syntax_pat kw"end"

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

notation_pat_part ::= (notation_pat_lit)     str
                    | (notation_pat_kw)      "@" kw"kw" str
                    | (notation_pat_name)    "@" kw"name"
                    | (notation_pat_cat)     name maybe_notation_pat_term_args
                    | (notation_pat_binding) name ":" "@" kw"binding" "(" name ")"

maybe_notation_pat_term_args ::= (maybe_notation_pat_term_args_none)
                               | (maybe_notation_pat_term_args_some) "(" notation_pat_term_args ")"

notation_pat_term_args ::= (notation_pat_term_args_one)  @name
                         | (notation_pat_term_args_many) @name notation_pat_term_args

grammar_category_command ::= (grammar_category) kw"grammar_category" name

tactic_command ::= (tactic) kw"tactic" name name prec_assoc "::=" grammar_pat kw"end"

attribute_command ::= (attribute) kw"attribute" name name prec_assoc "::=" grammar_pat kw"end"

grammar_pat ::= (grammar_pat_none)
              | (grammar_pat_many) grammar_pat_part grammar_pat

grammar_pat_part ::= (grammar_pat_part) maybe_label grammar_pat_part_core

maybe_label ::= (label_none)
              | (label_some) name ":"

grammar_pat_part_core ::= (core_lit)          str
                        | (core_kw)           "@" kw"kw" str
                        | (core_name)         "@" kw"name"
                        | (core_cat)          name
                        | (core_fragment)     "@" kw"fragment" "(" name ")"
                        | (core_any_fragment) "@" kw"any_fragment"
                        | (core_fact)         "@" kw"fact"

definition_command ::= (definition) kw"definition" notation_binding ":=" any_fragment kw"end"

// notation_binding is created from each notation command

axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"
theorem_command ::= (theorem) kw"theorem" name templates ":" hypotheses "|-" sentence kw"proof" tactic kw"qed"

templates ::= (template_none)
            | (template_many) template templates

template ::= (template) "[" template_bindings ":" template_cat "]"

template_cat ::= (template_cat_no_holes) name
               | (template_cat_holes)    name "(" cat_list ")"

cat_list ::= (cat_list_one)  name
           | (cat_list_many) name "," cat_list

template_bindings ::= (template_bindings_none)
                    | (template_bindings_many) notation_binding template_bindings

hypotheses ::= (hypotheses_none)
             | (hypotheses_many) hypothesis hypotheses

hypothesis ::= (hypothesis) "(" fact ")"

fact ::= (fact_assumption) kw"assume" sentence "|-" sentence
       | (fact_sentence)   sentence

// tactic syntax is now user-defined via grammar_category and tactic commands

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
        command_decl,
        command,
        maybe_attribute_anno,
        attribute_anno,
        attributes,
        module_command,
        syntax_cat_command,
        syntax_command,
        notation_command,
        definition_command,
        axiom_command,
        theorem_command,
        grammar_category_command,
        tactic_command,
        attribute_command,
        prec_assoc,
        maybe_prec,
        maybe_assoc,
        syntax_pat,
        syntax_pat_part,
        notation_pat,
        notation_pat_part,
        maybe_notation_pat_term_args,
        notation_pat_term_args,
        grammar_pat,
        grammar_pat_part,
        maybe_label,
        grammar_pat_part_core,
        notation_binding,
        templates,
        template,
        template_cat,
        cat_list,
        template_bindings,
        hypotheses,
        hypothesis,
        fact,
        any_fragment,
        name,
        str,
    }
}

builtin_rules! {
    BuiltinRules {
        name,
        str,
        command_decl,
        attribute_anno_none,
        attribute_anno_some,
        attribute_anno,
        attributes_one,
        attributes_many,
        module_command,
        syntax_cat_command,
        syntax_command,
        notation_command,
        definition_command,
        axiom_command,
        theorem_command,
        grammar_category_command,
        tactic_command,
        attribute_command,
        module,
        syntax_cat,
        syntax,
        grammar_category,
        tactic,
        attribute,
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
        syntax_pat_part_lit,
        notation,
        notation_pat_one,
        notation_pat_many,
        notation_pat_lit,
        notation_pat_kw,
        notation_pat_cat,
        notation_pat_name,
        notation_pat_binding,
        maybe_notation_pat_term_args_none,
        maybe_notation_pat_term_args_some,
        notation_pat_term_args_one,
        notation_pat_term_args_many,
        grammar_pat_none,
        grammar_pat_many,
        grammar_pat_part,
        label_none,
        label_some,
        core_lit,
        core_kw,
        core_name,
        core_cat,
        core_fragment,
        core_any_fragment,
        core_fact,
        definition,
        theorem,
        axiom,
        template_none,
        template_many,
        template,
        template_cat_no_holes,
        template_cat_holes,
        cat_list_one,
        cat_list_many,
        template_bindings_none,
        template_bindings_many,
        hypotheses_none,
        hypotheses_many,
        hypothesis,
        fact_assumption,
        fact_sentence,
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
    tactic_parse_cat: CategoryId<'ctx>,
    attribute_parse_cat: CategoryId<'ctx>,
    cats: &BuiltinCats<'ctx>,
) -> BuiltinRules<'ctx> {
    let sentence_cat = Category::new(
        *strings::SENTENCE,
        SyntaxCategorySource::FormalLang(formal_sentence_cat),
    );
    let sentence_cat = categories.alloc(*strings::SENTENCE, sentence_cat);
    state.use_cat(sentence_cat);

    macro_rules! rule {
        ($name:expr, $cat:expr, $parts:expr $(,)?) => {
            rule!($name, $cat, $parts, Associativity::default())
        };
        ($name:expr, $cat:expr, $parts:expr, $assoc:expr $(,)?) => {{
            let rule = rules.alloc(Rule::new(
                $name,
                $cat,
                ParseRuleSource::Builtin,
                RulePattern::new($parts, Precedence::default(), $assoc),
            ));
            state.use_rule(rule);
            rule
        }};
    }

    let rules = BuiltinRules {
        name: rule!(
            "name",
            cats.name,
            vec![RulePatternPart::Atom(ParseAtomPattern::Name)],
        ),
        str: rule!(
            "str",
            cats.str,
            vec![RulePatternPart::Atom(ParseAtomPattern::Str)],
        ),

        command_decl: rule!(
            "command_decl",
            cats.command_decl,
            vec![cat(cats.maybe_attribute_anno), cat(cats.command)],
        ),
        attribute_anno_none: rule!("attribute_anno_none", cats.maybe_attribute_anno, vec![]),
        attribute_anno_some: rule!(
            "attribute_anno_some",
            cats.maybe_attribute_anno,
            vec![cat(cats.attribute_anno)],
        ),
        attribute_anno: rule!(
            "attribute_anno",
            cats.attribute_anno,
            vec![
                lit(*strings::AT),
                lit(*strings::LEFT_BRACKET),
                cat(cats.attributes),
                lit(*strings::RIGHT_BRACKET),
            ],
        ),
        attributes_one: rule!(
            "attributes_one",
            cats.attributes,
            vec![cat(attribute_parse_cat)],
        ),
        attributes_many: rule!(
            "attributes_many",
            cats.attributes,
            vec![
                cat(attribute_parse_cat),
                lit(*strings::COMMA),
                cat(cats.attributes),
            ],
        ),
        module_command: rule!(
            "module_command",
            cats.command,
            vec![cat(cats.module_command)],
        ),
        syntax_cat_command: rule!(
            "syntax_cat_command",
            cats.command,
            vec![cat(cats.syntax_cat_command)],
        ),
        syntax_command: rule!(
            "syntax_command",
            cats.command,
            vec![cat(cats.syntax_command)],
        ),
        notation_command: rule!(
            "notation_command",
            cats.command,
            vec![cat(cats.notation_command)],
        ),
        definition_command: rule!(
            "definition_command",
            cats.command,
            vec![cat(cats.definition_command)],
        ),
        axiom_command: rule!("axiom_command", cats.command, vec![cat(cats.axiom_command)]),
        theorem_command: rule!(
            "theorem_command",
            cats.command,
            vec![cat(cats.theorem_command)],
        ),
        grammar_category_command: rule!(
            "grammar_category_command",
            cats.command,
            vec![cat(cats.grammar_category_command)],
        ),
        tactic_command: rule!(
            "tactic_command",
            cats.command,
            vec![cat(cats.tactic_command)],
        ),
        attribute_command: rule!(
            "attribute_command",
            cats.command,
            vec![cat(cats.attribute_command)],
        ),
        module: rule!(
            "module",
            cats.module_command,
            vec![kw(*strings::MODULE), cat(cats.name)],
        ),
        syntax_cat: rule!(
            "syntax_cat",
            cats.syntax_cat_command,
            vec![kw(*strings::SYNTAX_CAT), cat(cats.name)],
        ),
        syntax: rule!(
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
        grammar_category: rule!(
            "grammar_category",
            cats.grammar_category_command,
            vec![kw(*strings::GRAMMAR_CATEGORY), cat(cats.name)],
        ),
        tactic: rule!(
            "tactic",
            cats.tactic_command,
            vec![
                kw(*strings::TACTIC),
                cat(cats.name),
                cat(cats.name),
                cat(cats.prec_assoc),
                lit(*strings::BNF_REPLACE),
                cat(cats.grammar_pat),
                kw(*strings::END),
            ],
        ),
        attribute: rule!(
            "attribute",
            cats.attribute_command,
            vec![
                kw(*strings::ATTRIBUTE),
                cat(cats.name),
                cat(cats.name),
                cat(cats.prec_assoc),
                lit(*strings::BNF_REPLACE),
                cat(cats.grammar_pat),
                kw(*strings::END),
            ],
        ),
        notation: rule!(
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
        definition: rule!(
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
        axiom: rule!(
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
        theorem: rule!(
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
                cat(tactic_parse_cat),
                kw(*strings::QED),
            ],
        ),

        prec_assoc_none: rule!("prec_assoc_none", cats.prec_assoc, vec![]),
        prec_assoc_some: rule!(
            "prec_assoc_some",
            cats.prec_assoc,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.maybe_prec),
                cat(cats.maybe_assoc),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        prec_none: rule!("prec_none", cats.maybe_prec, vec![]),
        prec_some: rule!("prec_some", cats.maybe_prec, vec![num()]),
        assoc_none: rule!("assoc_none", cats.maybe_assoc, vec![]),
        assoc_left: rule!(
            "assoc_left",
            cats.maybe_assoc,
            vec![lit(*strings::LEFT_ARROW)],
        ),
        assoc_right: rule!(
            "assoc_left",
            cats.maybe_assoc,
            vec![lit(*strings::RIGHT_ARROW)],
        ),

        syntax_pat_one: rule!(
            "syntax_pat_one",
            cats.syntax_pat,
            vec![cat(cats.syntax_pat_part)],
        ),
        syntax_pat_many: rule!(
            "syntax_pat_many",
            cats.syntax_pat,
            vec![cat(cats.syntax_pat_part), cat(cats.syntax_pat)],
        ),

        syntax_pat_part_cat: rule!(
            "syntax_pat_part_cat",
            cats.syntax_pat_part,
            vec![cat(cats.name)],
        ),
        syntax_pat_part_binding: rule!(
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
        syntax_pat_part_lit: rule!(
            "syntax_pat_part_lit",
            cats.syntax_pat_part,
            vec![cat(cats.str)],
        ),

        notation_pat_one: rule!(
            "notation_pat_one",
            cats.notation_pat,
            vec![cat(cats.notation_pat_part)],
        ),
        notation_pat_many: rule!(
            "notation_pat_many",
            cats.notation_pat,
            vec![cat(cats.notation_pat_part), cat(cats.notation_pat)],
        ),
        notation_pat_lit: rule!(
            "notation_pat_lit",
            cats.notation_pat_part,
            vec![cat(cats.str)],
        ),
        notation_pat_kw: rule!(
            "notation_pat_kw",
            cats.notation_pat_part,
            vec![lit(*strings::AT), kw(*strings::KW), cat(cats.str)],
        ),
        notation_pat_name: rule!(
            "notation_pat_name",
            cats.notation_pat_part,
            vec![lit(*strings::AT), kw(*strings::NAME)],
        ),
        notation_pat_cat: rule!(
            "notation_pat_cat",
            cats.notation_pat_part,
            vec![cat(cats.name), cat(cats.maybe_notation_pat_term_args)],
        ),
        notation_pat_binding: rule!(
            "notation_pat_binding",
            cats.notation_pat_part,
            vec![
                cat(cats.name),
                lit(*strings::COLON),
                lit(*strings::AT),
                kw(*strings::BINDING),
                lit(*strings::LEFT_PAREN),
                cat(cats.name),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        maybe_notation_pat_term_args_none: rule!(
            "maybe_notation_pat_term_args_none",
            cats.maybe_notation_pat_term_args,
            vec![],
        ),
        maybe_notation_pat_term_args_some: rule!(
            "maybe_notation_pat_term_args_some",
            cats.maybe_notation_pat_term_args,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.notation_pat_term_args),
                lit(*strings::RIGHT_PAREN)
            ],
        ),
        notation_pat_term_args_one: rule!(
            "notation_pat_term_args_one",
            cats.notation_pat_term_args,
            vec![cat(cats.name)],
        ),
        notation_pat_term_args_many: rule!(
            "notation_pat_term_args_many",
            cats.notation_pat_term_args,
            vec![cat(cats.name), cat(cats.notation_pat_term_args)],
        ),

        grammar_pat_none: rule!("grammar_pat_none", cats.grammar_pat, vec![]),
        grammar_pat_many: rule!(
            "grammar_pat_many",
            cats.grammar_pat,
            vec![cat(cats.grammar_pat_part), cat(cats.grammar_pat)],
        ),
        grammar_pat_part: rule!(
            "grammar_pat_part",
            cats.grammar_pat_part,
            vec![cat(cats.maybe_label), cat(cats.grammar_pat_part_core)],
        ),

        label_none: rule!("label_none", cats.maybe_label, vec![]),
        label_some: rule!(
            "label_some",
            cats.maybe_label,
            vec![cat(cats.name), lit(*strings::COLON)],
        ),

        core_lit: rule!("core_lit", cats.grammar_pat_part_core, vec![cat(cats.str)]),
        core_kw: rule!(
            "core_kw",
            cats.grammar_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::KW), cat(cats.str)],
        ),
        core_name: rule!(
            "core_name",
            cats.grammar_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::NAME)],
        ),
        core_cat: rule!("core_cat", cats.grammar_pat_part_core, vec![cat(cats.name)]),
        core_fragment: rule!(
            "core_fragment",
            cats.grammar_pat_part_core,
            vec![
                lit(*strings::AT),
                kw(*strings::FRAGMENT),
                lit(*strings::LEFT_PAREN),
                cat(cats.name),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        core_any_fragment: rule!(
            "core_any_fragment",
            cats.grammar_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::ANY_FRAGMENT)],
        ),
        core_fact: rule!(
            "core_fact",
            cats.grammar_pat_part_core,
            vec![lit(*strings::AT), kw(*strings::FACT)],
        ),

        template_none: rule!("template_none", cats.templates, vec![]),
        template_many: rule!(
            "template_many",
            cats.templates,
            vec![cat(cats.template), cat(cats.templates)],
        ),

        template: rule!(
            "template",
            cats.template,
            vec![
                lit(*strings::LEFT_BRACKET),
                cat(cats.template_bindings),
                lit(*strings::COLON),
                cat(cats.template_cat),
                lit(*strings::RIGHT_BRACKET),
            ],
        ),
        template_cat_no_holes: rule!(
            "template_cat_no_holes",
            cats.template_cat,
            vec![cat(cats.name),],
        ),
        template_cat_holes: rule!(
            "template_cat_holes",
            cats.template_cat,
            vec![
                cat(cats.name),
                lit(*strings::LEFT_PAREN),
                cat(cats.cat_list),
                lit(*strings::RIGHT_PAREN)
            ],
        ),

        cat_list_one: rule!("cat_list_one", cats.cat_list, vec![cat(cats.name)]),
        cat_list_many: rule!(
            "cat_list_many",
            cats.cat_list,
            vec![cat(cats.name), lit(*strings::COMMA), cat(cats.cat_list)]
        ),

        template_bindings_none: rule!("template_bindings_none", cats.template_bindings, vec![]),
        template_bindings_many: rule!(
            "template_bindings_many",
            cats.template_bindings,
            vec![cat(cats.notation_binding), cat(cats.template_bindings)],
            Associativity::Left,
        ),

        hypotheses_none: rule!("hypotheses_none", cats.hypotheses, vec![]),
        hypotheses_many: rule!(
            "hypotheses_many",
            cats.hypotheses,
            vec![cat(cats.hypothesis), cat(cats.hypotheses)],
        ),

        hypothesis: rule!(
            "hypothesis",
            cats.hypothesis,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.fact),
                lit(*strings::RIGHT_PAREN),
            ],
        ),

        fact_assumption: rule!(
            "fact_assumption",
            cats.fact,
            vec![
                kw(*strings::ASSUME),
                cat(sentence_cat),
                lit(*strings::TURNSTILE),
                cat(sentence_cat),
            ],
        ),
        fact_sentence: rule!("fact_sentence", cats.fact, vec![cat(sentence_cat)]),
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
        ParseRuleSource::AnyFrag(formal_cat),
        RulePattern::new(
            vec![cat(parse_cat)],
            Precedence::default(),
            Associativity::default(),
        ),
    ));
    ctx.parse_state.use_rule(rule);

    // Create an annotated_name category for this formal category to support
    // optional category annotations like `x:term` for disambiguation.
    let annotated_name_cat_name = Ustr::from(&format!("annotated_name_{}", formal_cat.name()));
    let annotated_name_cat = Category::new(annotated_name_cat_name, SyntaxCategorySource::Builtin);
    let annotated_name_cat = ctx
        .arenas
        .parse_cats
        .alloc(annotated_name_cat_name, annotated_name_cat);
    ctx.parse_state.use_cat(annotated_name_cat);
    ctx.annotated_name_cats
        .insert(formal_cat, annotated_name_cat);

    // Rule 1: annotated_name_<cat> ::= name (no annotation)
    let plain_rule = ctx.arenas.parse_rules.alloc(Rule::new(
        format!("annotated_name_{}_plain", formal_cat.name()),
        annotated_name_cat,
        ParseRuleSource::Builtin,
        RulePattern::new(
            vec![cat(ctx.builtin_cats.name)],
            Precedence::default(),
            Associativity::default(),
        ),
    ));
    ctx.parse_state.use_rule(plain_rule);

    // Rule 2: annotated_name_<cat> ::= name ":" kw"<cat>" (with annotation)
    let annotated_rule = ctx.arenas.parse_rules.alloc(Rule::new(
        format!("annotated_name_{}_annotated", formal_cat.name()),
        annotated_name_cat,
        ParseRuleSource::Builtin,
        RulePattern::new(
            vec![
                cat(ctx.builtin_cats.name),
                lit(*strings::COLON),
                kw(formal_cat.name()),
            ],
            Precedence::default(),
            Associativity::default(),
        ),
    ));
    ctx.parse_state.use_rule(annotated_rule);
}

pub fn formal_rule_to_notation<'ctx>(
    rule: FormalSyntaxRuleId<'ctx>,
    syntax_cmd: CommandId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> (
    NotationPatternId<'ctx>,
    NotationBindingId<'ctx>,
    ScopeEntry<'ctx>,
) {
    fn to_notation<'ctx>(
        rule: FormalSyntaxRuleId<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> NotationPatternId<'ctx> {
        let mut args = Vec::new();
        for formal_part in rule.pattern().parts() {
            if let FormalSyntaxPatPart::Binding(cat) = *formal_part {
                args.push((args.len(), cat));
            }
        }

        let mut parts = Vec::new();

        for formal_part in rule.pattern().parts() {
            let part = match formal_part {
                FormalSyntaxPatPart::Cat(cat) => {
                    NotationPatternPart::Cat(NotationPatternPartCat::new(*cat, args.clone()))
                }
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
            NotationPatternSource::UserDeclared(rule.span()),
        );
        ctx.arenas.notations.alloc(pattern)
    }

    fn to_frag<'ctx>(rule: FormalSyntaxRuleId<'ctx>, ctx: &Ctx<'ctx>) -> PresFrag<'ctx> {
        let mut args = Vec::new();
        for formal_part in rule.pattern().parts() {
            if let FormalSyntaxPatPart::Binding(cat) = *formal_part {
                args.push(var_frag(args.len(), cat, ctx));
            }
        }

        let mut children = Vec::new();
        for formal_part in rule.pattern().parts() {
            if let FormalSyntaxPatPart::Cat(cat) = *formal_part {
                children.push(hole_frag(children.len(), cat, args.clone(), ctx));
            }
        }

        let frag_children = children.iter().map(|f| f.frag()).collect();
        let rule_app = FragRuleApplication::new(rule, args.len());
        let frag = Fragment::new(
            rule.cat(),
            FragHead::RuleApplication(rule_app),
            frag_children,
        );
        let frag = ctx.arenas.fragments.intern(frag);

        let pres = Pres::new(PresHead::FormalFrag(frag.head()), children);
        let pres = ctx.arenas.presentations.intern(pres);

        // This presentation is already formal so we don't need a formal reduction.
        PresFrag::new(frag, pres, pres)
    }

    let pattern = to_notation(rule, ctx);

    let binding = NotationBinding::new(pattern, Vec::new());
    let binding = ctx.arenas.notation_bindings.intern(binding);

    let frag = to_frag(rule, ctx);
    let scope_entry = ScopeEntry::new(frag, DefinitionSource::SyntaxCmd(syntax_cmd));

    (pattern, binding, scope_entry)
}

fn fragment_parse_rule_for_notation<'ctx>(
    notation: NotationPatternId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> RuleId<'ctx> {
    let mut parts = Vec::new();
    for notation_part in notation.0.parts() {
        let part = match notation_part {
            NotationPatternPart::Lit(lit_str) => {
                // Trim whitespace from literals for parsing, but preserve in presentation
                let trimmed = Ustr::from(lit_str.trim());
                lit(trimmed)
            }
            NotationPatternPart::Kw(kw_str) => kw(*kw_str),
            NotationPatternPart::Name => cat(ctx.builtin_cats.name),
            NotationPatternPart::Cat(part_cat) => {
                let cat = ctx.parse_state.cat_for_formal_cat(part_cat.cat());
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
    ctx: &Ctx<'ctx>,
) -> (RuleId<'ctx>, RuleId<'ctx>) {
    let mut parts = Vec::new();
    for notation_part in notation.0.parts() {
        let part = match notation_part {
            NotationPatternPart::Lit(lit_str) => {
                // Trim whitespace from literals for parsing, but preserve in presentation
                let trimmed = Ustr::from(lit_str.trim());
                lit(trimmed)
            }
            NotationPatternPart::Kw(kw_str) => kw(*kw_str),
            NotationPatternPart::Name => cat(ctx.builtin_cats.name),
            NotationPatternPart::Cat(_part_cat) => {
                // Use the formal category's annotated_name category to allow optional `:formal_cat` disambiguation
                cat(ctx.builtin_cats.notation_binding)
            }
            NotationPatternPart::Binding(formal_cat) => {
                // Use the formal category's annotated_name category to allow optional `:formal_cat` disambiguation
                cat(ctx.annotated_name_cats[formal_cat])
            }
        };
        parts.push(part)
    }

    let rule = |parts| {
        let parse_pat = RulePattern::new(parts, Precedence::default(), Associativity::default());

        let parse_rule = Rule::new(
            "notation_binding",
            ctx.builtin_cats.notation_binding,
            ParseRuleSource::Notation(notation),
            parse_pat,
        );

        ctx.arenas.parse_rules.alloc(parse_rule)
    };

    // Binding without the : cat at the end
    let unannotated = rule(parts.clone());

    // or with the : cat
    parts.push(lit(*strings::COLON));
    parts.push(kw(notation.cat().name()));
    let annotated = rule(parts);

    (unannotated, annotated)
}

pub fn add_parse_rules_for_notation<'ctx>(notation: NotationPatternId<'ctx>, ctx: &mut Ctx<'ctx>) {
    let fragment_rule = fragment_parse_rule_for_notation(notation, ctx);
    ctx.parse_state.use_rule(fragment_rule);

    let (binding_rule1, binding_rule2) = binding_parse_rule_for_notation(notation, ctx);
    ctx.parse_state.use_rule(binding_rule1);
    ctx.parse_state.use_rule(binding_rule2);
}

fn custom_grammar_rule_to_parse_rule<'ctx>(
    grammar_rule: CustomGrammarRuleId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> RuleId<'ctx> {
    let mut parts = Vec::new();
    for grammar_part in grammar_rule.pattern().parts() {
        use CustomGrammarPatPartCore as PatPart;

        let part = match grammar_part.part() {
            &PatPart::Lit(lit_str) => {
                // Trim whitespace from literals for parsing, but preserve in presentation
                let trimmed = Ustr::from(lit_str.trim());
                lit(trimmed)
            }
            PatPart::Kw(kw_str) => kw(*kw_str),
            PatPart::Name => cat(ctx.builtin_cats.name),
            PatPart::Cat(tactic_cat) => {
                let cat = ctx.parse_state.cat_for_tactic_cat(*tactic_cat);
                RulePatternPart::Cat(cat)
            }
            PatPart::Frag(cat_id) => cat(*cat_id),
            PatPart::AnyFrag => cat(ctx.builtin_cats.any_fragment),
            PatPart::Fact => cat(ctx.builtin_cats.fact),
        };
        parts.push(part);
    }

    let parse_pat = RulePattern::new(
        parts,
        grammar_rule.pattern().precedence(),
        grammar_rule.pattern().associativity(),
    );

    let parse_rule = Rule::new(
        grammar_rule.name(),
        ctx.parse_state.cat_for_tactic_cat(grammar_rule.cat()),
        ParseRuleSource::TacticRule(grammar_rule),
        parse_pat,
    );

    ctx.arenas.parse_rules.alloc(parse_rule)
}

pub fn add_parse_rules_for_custom_grammar_rule<'ctx>(
    grammar_rule: CustomGrammarRuleId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) {
    let parse_rule = custom_grammar_rule_to_parse_rule(grammar_rule, ctx);
    ctx.parse_state.use_rule(parse_rule);
}
