use crate::{
    context::Ctx,
    parse::parse_state::{
        CategoryId, ParseAtomPattern, ParseRuleSource, ParseState, Rule, RulePattern,
        RulePatternPart,
    },
    strings,
};
use ustr::Ustr;

macro_rules! builtin_cats {
    ($struct_name:ident { $( $name:ident ),* $(,)? }) => {
        pub struct $struct_name {
            $( pub $name: $crate::parse::parse_state::CategoryId, )*
        }

        impl $struct_name {
            pub fn new(ctx: &mut $crate::parse::parse_state::ParseState) -> Self {
                Self {
                    $( $name: ctx.new_builtin_cat(stringify!($name)), )*
                }
            }
        }
    };
}

macro_rules! builtin_rules {
    ($struct_name:ident { $( $name:ident ),* $(,)? }) => {
        pub struct $struct_name {
            $( pub $name: $crate::parse::parse_state::RuleId, )*
        }
    };
}

/*
Grammar of the Watson language:

command ::= (mod_command)        kw"module" name
          | (syntax_cat_command) kw"syntax_cat" name
          | (syntax_command)     kw"syntax" name name "::=" syntax_pat_list kw"end"
          | (macro_command)      kw"macro" name <category> "::=" macro_pat_list "=>" template(category) kw"end"
          | (axiom_command)      kw"axiom" name template ":" hypotheses "|-" sentence kw"end"
          | (theorem_command)    kw"theorem" name template ":" hypotheses "|-" sentence kw"proof" tactics kw"qed"

syntax_pat_list ::= (syntax_pat_list_one)  syntax_pat
                  | (syntax_pat_list_many) syntax_pat syntax_pat_list

syntax_pat ::= (syntax_pat_cat)     name
             | (syntax_pat_binding) "@" kw"binding" "(" name ")"
             | (syntax_pat_var)     "@" kw"var" "(" name ")"
             | (syntax_pat_lit)     str

macro_pat_list ::= (macro_pat_list_one)  macro_pat
                 | (macro_pat_list_many) macro_pat macro_pat_list

macro_pat ::= (macro_pat) macro_pat_binding macro_pat_kind

macro_pat_binding ::= (macro_pat_binding_empty)
                    | (macro_pat_binding_name)  "$" name ":"
macro_pat_kind ::= (macro_pat_kind_kw)       "@" kw"kw" str
                 | (macro_pat_kind_name)     "@" kw"name"
                 | (macro_pat_kind_lit)      str
                 | (macro_pat_kind_cat)      name
                 | (macro_pat_kind_template) "@" kw"template" "(" name ")"

templates ::= (template_none)
            | (template_many) `template template

template ::= (template) "[" name maybe_template_params ":" name "]"

maybe_template_params ::= (maybe_template_params_none)
                        | (maybe_template_params_some) "(" template_params ")"
template_params ::= (template_params_one)  template_arg
                  | (template_params_many) template_arg "," template_params
template_arg ::= (template_arg) name

hypotheses ::= (hypotheses_none)
             | (hypotheses_many) hypothesis hypotheses

hypothesis ::= (hypothesis) "(" fact ")"

fact ::= (fact_assumption) kw"assume" sentence "|-" sentence
       | (fact_sentence)   sentence

tactics ::= (tactics_none)
          | (tactics_have) kw"have" fact tactics ";" tactics
          | (tactics_by)   kw"by" name template_instantiations tactics
          | (tactics_todo) kw"todo" tactics

template_instantiations ::= (template_instantiations_none)
                          | (template_instantiations_many) template_instantiation template_instantiations

template_instantiation ::= "[" <formal_cat> "]"
*/

builtin_cats! {
    BuiltinCats {
        command,
        syntax_pat_list,
        syntax_pat,
        macro_pat_list,
        macro_pat,
        macro_pat_binding,
        macro_pat_kind,
        template,
        maybe_template_params,
        template_params,
        template_arg,
        hypotheses,
        hypothesis,
        fact,
        tactics,
        template_instantiations,
        template_instantiation,
    }
}

builtin_rules! {
    BuiltinRules {
        mod_command,
        syntax_cat_command,
        syntax_command,
        axiom_command,
        theorem_command,
        syntax_pat_list_one,
        syntax_pat_list_many,
        syntax_pat_cat,
        syntax_pat_binding,
        syntax_pat_var,
        syntax_pat_lit,
        macro_pat_list_one,
        macro_pat_list_many,
        macro_pat,
        macro_pat_binding_empty,
        macro_pat_binding_name,
        macro_pat_kind_kw,
        macro_pat_kind_name,
        macro_pat_kind_lit,
        macro_pat_kind_cat,
        macro_pat_kind_template,
        template_none,
        template_many,
        template,
        maybe_template_params_none,
        maybe_template_params_some,
        template_params_one,
        template_params_many,
        template_arg,
        hypotheses_none,
        hypotheses_many,
        hypothesis,
        fact_assumption,
        fact_sentence,
        tactics_none,
        tactics_have,
        tactics_by,
        tactics_todo,
        template_instantiations_none,
        template_instantiations_many,
    }
}

fn kw(kw: Ustr) -> RulePatternPart {
    RulePatternPart::Atom(ParseAtomPattern::Kw(kw))
}

fn lit(lit: Ustr) -> RulePatternPart {
    RulePatternPart::Atom(ParseAtomPattern::Lit(lit))
}

fn name() -> RulePatternPart {
    RulePatternPart::Atom(ParseAtomPattern::Name)
}

fn str() -> RulePatternPart {
    RulePatternPart::Atom(ParseAtomPattern::Str)
}

fn cat(cat: CategoryId) -> RulePatternPart {
    RulePatternPart::Cat(cat)
}

fn cat_template(cat: CategoryId) -> RulePatternPart {
    RulePatternPart::TempCat(cat)
}

pub fn add_builtin_rules(parse_state: &mut ParseState, cats: &BuiltinCats) -> BuiltinRules {
    let mut rule = |name: &str, cat, parts| {
        parse_state.add_rule(Rule::new(
            name,
            cat,
            ParseRuleSource::Builtin,
            RulePattern::new(parts),
        ))
    };

    BuiltinRules {
        mod_command: rule(
            "mod_command",
            cats.command,
            vec![kw(*strings::MODULE), name()],
        ),
        syntax_cat_command: rule(
            "syntax_cat_command",
            cats.command,
            vec![kw(*strings::SYNTAX_CAT), name()],
        ),
        syntax_command: rule(
            "syntax_command",
            cats.command,
            vec![
                kw(*strings::SYNTAX),
                name(),
                name(),
                lit(*strings::BNF_REPLACE),
                cat(cats.syntax_pat_list),
                kw(*strings::END),
            ],
        ),
        axiom_command: rule(
            "axiom_command",
            cats.command,
            vec![
                kw(*strings::AXIOM),
                name(),
                cat(cats.template),
                lit(*strings::COLON),
                cat(cats.hypotheses),
                kw(*strings::TURNSTILE),
                cat(cats.fact),
                kw(*strings::END),
            ],
        ),
        theorem_command: rule(
            "theorem_command",
            cats.command,
            vec![
                kw(*strings::THEOREM),
                name(),
                cat(cats.template),
                lit(*strings::COLON),
                cat(cats.hypotheses),
                kw(*strings::TURNSTILE),
                cat(cats.fact),
                kw(*strings::PROOF),
                cat(cats.tactics),
                kw(*strings::QED),
            ],
        ),

        syntax_pat_list_one: rule(
            "syntax_pat_list_one",
            cats.syntax_pat_list,
            vec![cat(cats.syntax_pat)],
        ),
        syntax_pat_list_many: rule(
            "syntax_pat_list_many",
            cats.syntax_pat_list,
            vec![cat(cats.syntax_pat), cat(cats.syntax_pat_list)],
        ),

        syntax_pat_cat: rule("syntax_pat_cat", cats.syntax_pat, vec![name()]),
        syntax_pat_binding: rule(
            "syntax_pat_binding",
            cats.syntax_pat,
            vec![
                lit(*strings::AT),
                kw(*strings::BINDING),
                lit(*strings::LEFT_PAREN),
                name(),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        syntax_pat_var: rule(
            "syntax_pat_var",
            cats.syntax_pat,
            vec![
                lit(*strings::AT),
                kw(*strings::VARIABLE),
                lit(*strings::LEFT_PAREN),
                name(),
                lit(*strings::RIGHT_PAREN),
            ],
        ),
        syntax_pat_lit: rule("syntax_pat_lit", cats.syntax_pat, vec![str()]),

        macro_pat_list_one: rule(
            "macro_pat_list_one",
            cats.macro_pat_list,
            vec![cat(cats.macro_pat)],
        ),
        macro_pat_list_many: rule(
            "macro_pat_list_many",
            cats.macro_pat_list,
            vec![cat(cats.macro_pat), cat(cats.macro_pat_list)],
        ),

        macro_pat: rule(
            "macro_pat",
            cats.macro_pat,
            vec![cat(cats.macro_pat_binding), cat(cats.macro_pat_kind)],
        ),
        macro_pat_binding_empty: rule("macro_pat_binding_empty", cats.macro_pat_binding, vec![]),
        macro_pat_binding_name: rule(
            "macro_pat_binding_name",
            cats.macro_pat_binding,
            vec![lit(*strings::DOLLAR), name(), lit(*strings::COLON)],
        ),
        macro_pat_kind_kw: rule(
            "macro_pat_kind_kw",
            cats.macro_pat_kind,
            vec![lit(*strings::AT), kw(*strings::KW), str()],
        ),
        macro_pat_kind_name: rule(
            "macro_pat_kind_name",
            cats.macro_pat_kind,
            vec![lit(*strings::AT), kw(*strings::NAME)],
        ),
        macro_pat_kind_lit: rule("macro_pat_kind_lit", cats.macro_pat_kind, vec![str()]),
        macro_pat_kind_cat: rule("macro_pat_kind_cat", cats.macro_pat_kind, vec![name()]),
        macro_pat_kind_template: rule(
            "macro_pat_kind_template",
            cats.macro_pat_kind,
            vec![
                lit(*strings::AT),
                kw(*strings::TEMPLATE),
                lit(*strings::LEFT_PAREN),
                name(),
                lit(*strings::RIGHT_PAREN),
            ],
        ),

        template_none: rule("template_none", cats.template, vec![]),
        template_many: rule(
            "template_many",
            cats.template,
            vec![lit(*strings::TEMPLATE), cat(cats.template)],
        ),

        template: rule(
            "template",
            cats.template,
            vec![
                lit(*strings::LEFT_BRACKET),
                name(),
                cat(cats.maybe_template_params),
                lit(*strings::COLON),
                name(),
                lit(*strings::RIGHT_BRACKET),
            ],
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
            vec![cat(cats.template_arg)],
        ),
        template_params_many: rule(
            "template_params_many",
            cats.template_params,
            vec![
                cat(cats.template_arg),
                lit(*strings::COMMA),
                cat(cats.template_params),
            ],
        ),
        template_arg: rule("template_arg", cats.template_arg, vec![name()]),

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
                cat(cats.fact),
                lit(*strings::TURNSTILE),
                cat(cats.fact),
            ],
        ),
        fact_sentence: rule("fact_sentence", cats.fact, vec![cat(cats.syntax_pat)]),

        tactics_none: rule("tactics_none", cats.tactics, vec![]),
        tactics_have: rule(
            "tactics_have",
            cats.tactics,
            vec![
                kw(*strings::HAVE),
                cat(cats.fact),
                cat(cats.tactics),
                lit(*strings::SEMICOLON),
                cat(cats.tactics),
            ],
        ),
        tactics_by: rule(
            "tactics_by",
            cats.tactics,
            vec![
                kw(*strings::BY),
                name(),
                cat(cats.template_instantiations),
                cat(cats.tactics),
            ],
        ),
        tactics_todo: rule(
            "tactics_todo",
            cats.tactics,
            vec![kw(*strings::TODO), cat(cats.tactics)],
        ),

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
    }

    // TODO: template_instantiation
}

pub fn add_builtin_syntax_for_cat(for_cat: CategoryId, ctx: &mut Ctx) {
    ctx.parse_state.add_rule(Rule::new(
        "macro_command",
        ctx.builtin_cats.command,
        ParseRuleSource::Builtin,
        RulePattern::new(vec![
            kw(*strings::MACRO),
            kw(ctx.parse_state[for_cat].name()),
            lit(*strings::BNF_REPLACE),
            cat(ctx.builtin_cats.macro_pat_list),
            lit(*strings::FAT_ARROW),
            cat_template(for_cat),
            kw(*strings::END),
        ]),
    ));
}
