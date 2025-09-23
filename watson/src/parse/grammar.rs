use crate::{
    context::Ctx,
    parse::parse_state::{
        CategoryId, ParseAtomPattern, ParseRuleSource, ParseState, Rule, RulePattern,
        RulePatternPart,
    },
    semant::formal_syntax::{
        FormalSyntax, FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId,
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

command ::= (module_command)     module_command
          | (syntax_cat_command) syntax_cat_command
          | (syntax_command)     syntax_command
          | (macro_command)      macro_command
          | (axiom_command)      axiom_command
          | (theorem_command)    theorem_command

module_command ::= (module) kw"module" name

syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name
syntax_command ::= (syntax) kw"syntax" name name "::=" syntax_pat_list kw"end"

syntax_pat ::= (syntax_pat_one)  syntax_pat_part
             | (syntax_pat_many) syntax_pat_part syntax_pat

syntax_pat_part ::= (syntax_pat_cat)     name
                  | (syntax_pat_binding) "@" kw"binding" "(" name ")"
                  | (syntax_pat_var)     "@" kw"variable" "(" name ")"
                  | (syntax_pat_lit)     str

macro_command ::= (macro) kw"macro" name macro_replacement kw"end"

macro_replacement ::= (macro_replacement) <category> "::=" macro_pat_list "=>" template(category)

macro_pat ::= (macro_pat_one)  macro_pat_part
            | (macro_pat_many) macro_pat_part macro_pat

macro_pat_part ::= (macro_pat_part) macro_pat_binding macro_pat_kind

macro_pat_binding ::= (macro_pat_binding_empty)
                    | (macro_pat_binding_name)  "$" name ":"
macro_pat_kind ::= (macro_pat_kind_kw)       "@" kw"kw" str
                 | (macro_pat_kind_lit)      str
                 | (macro_pat_kind_cat)      name
                 | (macro_pat_kind_template) "@" kw"template" "(" name ")"

axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"
theorem_command ::= (theorem) kw"theorem" name templates ":" hypotheses "|-" sentence kw"proof" tactics kw"qed"

templates ::= (template_none)
            | (template_many) template templates

template ::= (template) "[" name maybe_template_params ":" name "]"

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
        macro_command,
        axiom_command,
        theorem_command,
        syntax_pat,
        syntax_pat_part,
        macro_replacement,
        macro_pat,
        macro_pat_part,
        macro_pat_binding,
        macro_pat_kind,
        templates,
        template,
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
        macro_command,
        axiom_command,
        theorem_command,
        module,
        syntax_cat,
        syntax,
        syntax_pat_one,
        syntax_pat_many,
        syntax_pat_part_cat,
        syntax_pat_part_binding,
        syntax_pat_part_var,
        syntax_pat_part_lit,
        macro_r,
        macro_pat_one,
        macro_pat_many,
        macro_pat_part,
        macro_pat_binding_empty,
        macro_pat_binding_name,
        macro_pat_kind_kw,
        macro_pat_kind_lit,
        macro_pat_kind_cat,
        macro_pat_kind_template,
        theorem,
        axiom,
        template_none,
        template_many,
        template,
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

fn kw(kw: Ustr) -> RulePatternPart {
    RulePatternPart::Atom(ParseAtomPattern::Kw(kw))
}

fn lit(lit: Ustr) -> RulePatternPart {
    RulePatternPart::Atom(ParseAtomPattern::Lit(lit))
}

fn cat(cat: CategoryId) -> RulePatternPart {
    RulePatternPart::Cat {
        id: cat,
        template: false,
    }
}

fn cat_template(cat: CategoryId) -> RulePatternPart {
    RulePatternPart::Cat {
        id: cat,
        template: true,
    }
}

pub fn add_builtin_rules(
    parse_state: &mut ParseState,
    formal_syntax: &FormalSyntax,
    cats: &BuiltinCats,
) -> BuiltinRules {
    let sentence_cat =
        parse_state.new_formal_lang_cat(*strings::SENTENCE, formal_syntax.sentence_cat());

    let mut rule = |name: &str, cat, parts| {
        parse_state.add_rule(Rule::new(
            name,
            cat,
            ParseRuleSource::Builtin,
            RulePattern::new(parts),
        ))
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
        macro_command: rule("macro_command", cats.command, vec![cat(cats.macro_command)]),
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
                lit(*strings::BNF_REPLACE),
                cat(cats.syntax_pat),
                kw(*strings::END),
            ],
        ),
        macro_r: rule(
            "macro_r",
            cats.macro_command,
            vec![
                kw(*strings::MACRO),
                cat(cats.name),
                cat(cats.macro_replacement),
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

        macro_pat_one: rule(
            "macro_pat_one",
            cats.macro_pat,
            vec![cat(cats.macro_pat_part)],
        ),
        macro_pat_many: rule(
            "macro_pat_many",
            cats.macro_pat,
            vec![cat(cats.macro_pat_part), cat(cats.macro_pat)],
        ),

        macro_pat_part: rule(
            "macro_pat_part",
            cats.macro_pat_part,
            vec![cat(cats.macro_pat_binding), cat(cats.macro_pat_kind)],
        ),
        macro_pat_binding_empty: rule("macro_pat_binding_empty", cats.macro_pat_binding, vec![]),
        macro_pat_binding_name: rule(
            "macro_pat_binding_name",
            cats.macro_pat_binding,
            vec![lit(*strings::DOLLAR), cat(cats.name), lit(*strings::COLON)],
        ),
        macro_pat_kind_kw: rule(
            "macro_pat_kind_kw",
            cats.macro_pat_kind,
            vec![lit(*strings::AT), kw(*strings::KW), cat(cats.str)],
        ),
        macro_pat_kind_lit: rule(
            "macro_pat_kind_lit",
            cats.macro_pat_kind,
            vec![cat(cats.str)],
        ),
        macro_pat_kind_cat: rule(
            "macro_pat_kind_cat",
            cats.macro_pat_kind,
            vec![cat(cats.name)],
        ),
        macro_pat_kind_template: rule(
            "macro_pat_kind_template",
            cats.macro_pat_kind,
            vec![
                lit(*strings::AT),
                kw(*strings::TEMPLATE),
                lit(*strings::LEFT_PAREN),
                cat(cats.name),
                lit(*strings::RIGHT_PAREN),
            ],
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
                cat(cats.name),
                cat(cats.maybe_template_params),
                lit(*strings::COLON),
                cat(cats.name),
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
                cat(cats.tactic),
            ],
        ),
        tactic_todo: rule(
            "tactic_todo",
            cats.tactic,
            vec![kw(*strings::TODO), cat(cats.tactic)],
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

pub fn add_builtin_syntax_for_cat(for_cat: CategoryId, ctx: &mut Ctx) {
    ctx.parse_state.add_rule(Rule::new(
        "macro_replacement",
        ctx.builtin_cats.macro_replacement,
        ParseRuleSource::Builtin,
        RulePattern::new(vec![
            kw(ctx.parse_state[for_cat].name()),
            lit(*strings::BNF_REPLACE),
            cat(ctx.builtin_cats.macro_pat),
            lit(*strings::FAT_ARROW),
            cat_template(for_cat),
        ]),
    ));

    ctx.parse_state.add_rule(Rule::new(
        "macro_binding",
        for_cat,
        ParseRuleSource::Builtin,
        RulePattern::new(vec![RulePatternPart::Atom(ParseAtomPattern::MacroBinding)]),
    ));
}

pub fn add_builtin_syntax_for_formal_cat(formal_cat: FormalSyntaxCatId, ctx: &mut Ctx) {
    let macro_cat = ctx.parse_state.cat_for_formal_cat(formal_cat);

    ctx.parse_state.add_rule(Rule::new(
        "template_instantiation",
        macro_cat,
        ParseRuleSource::Builtin,
        RulePattern::new(vec![
            cat(ctx.builtin_cats.name),
            cat(ctx.builtin_cats.maybe_shorthand_args),
        ]),
    ));

    ctx.parse_state.add_rule(Rule::new(
        "any_fragment",
        ctx.builtin_cats.any_fragment,
        ParseRuleSource::Builtin,
        RulePattern::new(vec![cat(macro_cat)]),
    ));
}

pub fn add_formal_syntax_rule(rule_id: FormalSyntaxRuleId, ctx: &mut Ctx) {
    let rule = &ctx.formal_syntax[rule_id];

    let mut parts = Vec::new();
    for formal_part in rule.pattern().parts() {
        let part = match formal_part {
            FormalSyntaxPatPart::Cat(formal_cat) => {
                let cat = ctx.parse_state.cat_for_formal_cat(*formal_cat);
                RulePatternPart::Cat {
                    id: cat,
                    template: false,
                }
            }
            FormalSyntaxPatPart::Binding(_) | FormalSyntaxPatPart::Var(_) => {
                cat(ctx.builtin_cats.name)
            }
            FormalSyntaxPatPart::Lit(lit_str) => lit(*lit_str),
        };
        parts.push(part);
    }

    let parse_rule_pattern = RulePattern::new(parts);

    let parse_rule = Rule::new(
        "formal_syntax_rule",
        ctx.parse_state.cat_for_formal_cat(rule.cat()),
        ParseRuleSource::FormalLang(rule_id),
        parse_rule_pattern,
    );

    ctx.parse_state.add_rule(parse_rule);
}
