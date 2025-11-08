use std::vec;

use crate::{
    context::{
        Ctx,
        arena::{NamedArena, PlainArena},
    },
    parse::parse_state::{
        Associativity, Category, CategoryId, ParseAtomPattern, ParseRuleSource, ParseState,
        Precedence, Rule, RuleId, RulePattern, RulePatternPart, SyntaxCategorySource,
    },
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
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

command ::= (module_command)     module_command
          | (syntax_cat_command) syntax_cat_command
          | (syntax_command)     syntax_command
          | (axiom_command)      axiom_command
          | (theorem_command)    theorem_command

module_command ::= (module) kw"module" name

syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name
syntax_command ::= (syntax) kw"syntax" name name prec_assoc "::=" syntax_pat_list kw"end"

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
                  | (syntax_pat_var)     "@" kw"variable" "(" name ")"
                  | (syntax_pat_lit)     str

axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"
theorem_command ::= (theorem) kw"theorem" name templates ":" hypotheses "|-" sentence kw"proof" tactics kw"qed"

templates ::= (template_none)
            | (template_many) template templates

template ::= (template) "[" template_names ":" name "]"

template_names ::= (template_names_none)
                 | (template_names_many) template_name template_names

template_name ::= (template_name) name maybe_template_params

maybe_template_params ::= (maybe_template_params_none)
                        | (maybe_template_params_some) "(" template_params ")"
template_params ::= (template_params_one)  template_param
                  | (template_params_many) template_param "," template_params
template_param ::= (template_param) name

hypotheses ::= (hypotheses_none)
             | (hypotheses_many) hypothesis hypotheses

hypothesis ::= (hypothesis) "(" fact ")"

fact ::= (fact_assumption) kw"assume" sentence "|-" sentence
       | (fact_sentence)   sentence

tactic ::= (tactic_none)
         | (tactic_have) kw"have" fact tactics ";" tactics
         | (tactic_by)   kw"by" name template_instantiations
         | (tactic_todo) kw"todo"

template_instantiations ::= (template_instantiations_none)
                          | (template_instantiations_many) template_instantiation template_instantiations

template_instantiation ::= "[" any_fragment "]"

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
        axiom_command,
        theorem_command,
        prec_assoc,
        maybe_prec,
        maybe_assoc,
        syntax_pat,
        syntax_pat_part,
        templates,
        template,
        template_names,
        template_name,
        maybe_template_params,
        template_params,
        template_param,
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
        axiom_command,
        theorem_command,
        module,
        syntax_cat,
        syntax,
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
        theorem,
        axiom,
        template_none,
        template_many,
        template,
        template_names_none,
        template_names_many,
        template_name,
        maybe_template_params_none,
        maybe_template_params_some,
        template_params_one,
        template_params_many,
        template_param,
        hypotheses_none,
        hypotheses_many,
        hypothesis,
        fact_assumption,
        fact_sentence,
        tactic_none,
        tactic_have,
        tactic_by,
        tactic_todo,
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

    BuiltinRules {
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
        axiom_command: rule("axiom_command", cats.command, vec![cat(cats.axiom_command)]),
        theorem_command: rule(
            "theorem_command",
            cats.command,
            vec![cat(cats.theorem_command)],
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
                cat(cats.template_names),
                lit(*strings::COLON),
                cat(cats.name),
                lit(*strings::RIGHT_BRACKET),
            ],
        ),

        template_names_none: rule("template_names_none", cats.template_names, vec![]),
        template_names_many: rule(
            "template_names_many",
            cats.template_names,
            vec![cat(cats.template_name), cat(cats.template_names)],
        ),

        template_name: rule(
            "template_name",
            cats.template_name,
            vec![cat(cats.name), cat(cats.maybe_template_params)],
        ),

        maybe_template_params_none: rule(
            "maybe_template_params_none",
            cats.maybe_template_params,
            vec![],
        ),
        maybe_template_params_some: rule(
            "maybe_template_params_some",
            cats.maybe_template_params,
            vec![
                lit(*strings::LEFT_PAREN),
                cat(cats.template_params),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        template_params_one: rule(
            "template_params_one",
            cats.template_params,
            vec![cat(cats.template_param)],
        ),
        template_params_many: rule(
            "template_params_many",
            cats.template_params,
            vec![
                cat(cats.template_param),
                lit(*strings::COMMA),
                cat(cats.template_params),
            ],
        ),
        template_param: rule("template_param", cats.template_param, vec![cat(cats.name)]),

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

        tactic_none: rule("tactic_none", cats.tactic, vec![]),
        tactic_have: rule(
            "tactic_have",
            cats.tactic,
            vec![
                kw(*strings::HAVE),
                cat(cats.fact),
                cat(cats.tactic),
                lit(*strings::SEMICOLON),
                cat(cats.tactic),
            ],
        ),
        tactic_by: rule(
            "tactic_by",
            cats.tactic,
            vec![
                kw(*strings::BY),
                cat(cats.name),
                cat(cats.template_instantiations),
            ],
        ),
        tactic_todo: rule("tactic_todo", cats.tactic, vec![kw(*strings::TODO)]),

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
    }
}

pub fn add_builtin_syntax_for_formal_cat<'ctx>(
    formal_cat: FormalSyntaxCatId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) {
    let macro_cat = ctx.parse_state.cat_for_formal_cat(formal_cat);

    let rule = ctx.arenas.parse_rules.alloc(Rule::new(
        "template_instantiation",
        macro_cat,
        ParseRuleSource::Builtin,
        RulePattern::new(
            vec![
                cat(ctx.builtin_cats.name),
                cat(ctx.builtin_cats.maybe_shorthand_args),
            ],
            Precedence::default(),
            Associativity::default(),
        ),
    ));
    ctx.parse_state.use_rule(rule);

    let rule = ctx.arenas.parse_rules.alloc(Rule::new(
        "any_fragment",
        ctx.builtin_cats.any_fragment,
        ParseRuleSource::Builtin,
        RulePattern::new(
            vec![cat(macro_cat)],
            Precedence::default(),
            Associativity::default(),
        ),
    ));
    ctx.parse_state.use_rule(rule);
}

pub fn add_formal_syntax_rule<'ctx>(rule: FormalSyntaxRuleId<'ctx>, ctx: &mut Ctx<'ctx>) {
    let mut parts = Vec::new();
    for formal_part in rule.0.pattern().parts() {
        let part = match formal_part {
            FormalSyntaxPatPart::Cat(formal_cat) => {
                let cat = ctx.parse_state.cat_for_formal_cat(*formal_cat);
                RulePatternPart::Cat(cat)
            }
            FormalSyntaxPatPart::Binding(_) | FormalSyntaxPatPart::Var(_) => {
                cat(ctx.builtin_cats.name)
            }
            FormalSyntaxPatPart::Lit(lit_str) => lit(*lit_str),
        };
        parts.push(part);
    }

    let parse_rule_pattern = RulePattern::new(
        parts,
        rule.pattern().precedence(),
        rule.pattern().associativity(),
    );

    let parse_rule = Rule::new(
        "formal_syntax_rule",
        ctx.parse_state.cat_for_formal_cat(rule.cat()),
        ParseRuleSource::FormalLang(rule),
        parse_rule_pattern,
    );

    let rule = ctx.arenas.parse_rules.alloc(parse_rule);
    ctx.parse_state.use_rule(rule);
}
