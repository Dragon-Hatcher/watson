macro_rules! builtin_cats {
    ($struct_name:ident { $( $name:ident ),* $(,)? }) => {
        pub struct $struct_name {
            $( pub $name: $crate::parse::parse_tree::CategoryId, )*
        }

        impl $struct_name {
            pub fn new(ctx: &mut $crate::parse::parse_tree::ParseForest) -> Self {
                Self {
                    $( $name: ctx.new_builtin_cat(stringify!($name)), )*
                }
            }
        }
    };
}

macro_rules! builtin_rules {
    ($struct_name:ident { $( $name:ident : $cat:ident ),* $(,)? }) => {
        pub struct $struct_name {
            $( pub $name: $crate::parse::parse_tree::RuleId, )*
        }

        impl $struct_name {
            pub fn new(ctx: &mut $crate::parse::parse_tree::ParseForest, cats: &$crate::parse::builtin::BuiltinCats) -> Self {
                Self {
                    $( $name: ctx.new_builtin_rule(stringify!($name), cats.$cat), )*
                }
            }
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
        mod_command: command,
        syntax_cat_command: command,
        syntax_command: command,
        macro_command: command,
        axiom_command: command,
        theorem_command: command,
        syntax_pat_list_one: syntax_pat_list,
        syntax_pat_list_many: syntax_pat_list,
        syntax_pat_cat: syntax_pat,
        syntax_pat_binding: syntax_pat,
        syntax_pat_var: syntax_pat,
        syntax_pat_lit: syntax_pat,
        macro_pat_list_one: macro_pat_list,
        macro_pat_list_many: macro_pat_list,
        macro_pat: macro_pat,
        macro_pat_binding_empty: macro_pat_binding,
        macro_pat_binding_name: macro_pat_binding,
        macro_pat_kind_kw: macro_pat_kind,
        macro_pat_kind_name: macro_pat_kind,
        macro_pat_kind_lit: macro_pat_kind,
        macro_pat_kind_cat: macro_pat_kind,
        macro_pat_kind_template: macro_pat_kind,
        template_none: maybe_template_params,
        template_many: maybe_template_params,
        template: template,
        maybe_template_params_none: maybe_template_params,
        maybe_template_params_some: maybe_template_params,
        template_params_one: template_params,
        template_params_many: template_params,
        template_arg: template_arg,
        hypotheses_none: hypotheses,
        hypotheses_many: hypotheses,
        hypothesis: hypothesis,
        fact_assumption: fact,
        fact_sentence: fact,
        tactics_none: tactics,
        tactics_have: tactics,
        tactics_by: tactics,
        tactics_todo: tactics,
        template_instantiations_none: template_instantiations,
        template_instantiations_many: template_instantiations,
        template_instantiation: template_instantiation,
    }
}
