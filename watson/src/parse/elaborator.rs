use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, DiagnosticSpan, WResult},
    parse::{
        SourceId,
        parse_state::{Associativity, ParseRuleSource, Precedence, SyntaxCategorySource},
        parse_tree::{ParseTreeChildren, ParseTreeId},
        source_cache::{SourceDecl, source_id_to_path},
    },
    semant::{
        attributes::Attribute,
        commands::CommandId,
        custom_grammar::{
            inst::{CustomGrammarInst, CustomGrammarInstPart, SpannedStr},
            syntax::{
                CustomGrammarCat, CustomGrammarCatId, CustomGrammarPat, CustomGrammarPatPart,
                CustomGrammarPatPartCore, CustomGrammarRule, CustomGrammarRuleId,
            },
        },
        formal_syntax::{
            FormalSyntaxCat, FormalSyntaxCatId, FormalSyntaxPat, FormalSyntaxPatPart,
            FormalSyntaxRule, FormalSyntaxRuleId,
        },
        notation::{
            NotationBinding, NotationBindingId, NotationPattern, NotationPatternId,
            NotationPatternPart, NotationPatternPartCat, NotationPatternSource,
            NotationSignatureHole,
        },
        parse_fragment::{UnresolvedAnyFrag, UnresolvedFact, UnresolvedFrag, parse_fragment},
        presentation::PresFrag,
        scope::{DefinitionSource, Scope, ScopeEntry},
        tactic::unresolved_proof::UnresolvedProof,
        theorems::{PresFact, Template, TheoremId, TheoremStatement, add_templates_to_scope},
    },
    strings,
};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};
use ustr::Ustr;

// Reserved Luau type names that cannot be used as tactic category names
const RESERVED_LUAU_TYPES: &[&str] = &[
    "Span",
    "SpannedString",
    "UnResFrag",
    "UnResFragMeta",
    "UnResAnyFrag",
    "UnResFact",
    "FormalCat",
    "FormalCatMeta",
    "Frag",
    "FragMeta",
    "Fact",
    "FactMeta",
    "Binding",
    "BindingMeta",
    "Scope",
    "ScopeMeta",
    "ThmTemplate",
    "Theorem",
    "TheoremMeta",
    "ProofState",
    "TacticInfo",
    "DiagnosticSpan",
    "Diagnostic",
    "DiagnosticMeta",
    "VFunction",
    "VFunctionMeta",
    "VPredicate",
    "VPredicateMeta",
    "VTerm",
    "VTermMeta",
    "VFormula",
    "VFormulaMeta",
    "VOptions",
    "VOptionsMeta",
    "VProblem",
    "VProblemMeta",
];

macro_rules! failed_to_match_builtin {
    ($rule:expr, $ctx:expr) => {
        panic!("Failed to match builtin parse tree: {}", $rule.name());
    };
}

macro_rules! match_rule {
    (($ctx:expr, $tree_id:expr) => $($rule:ident ::= [$($child:ident),*] => $body:expr),+ $(,)?) => {{
        let tree = $tree_id;
        let children = expect_unambiguous(tree)?;
        $(
            if children.rule() == $ctx.builtin_rules.$rule {
                let [$($child),*] = children.children() else {
                    failed_to_match_builtin!(children.rule(), $ctx);
                };
                $(
                    let $child = *$child;
                )*
                $body
            } else
        )*
        {
            failed_to_match_builtin!(children.rule(), $ctx);
        }
    }}
}

#[allow(clippy::enum_variant_names)]
pub enum ElaborateAction<'ctx> {
    NewSource(SourceId),
    NewFormalCat(FormalSyntaxCatId<'ctx>),
    NewFormalRule(FormalSyntaxRuleId<'ctx>),
    NewNotation(NotationPatternId<'ctx>),
    NewDefinition(Scope<'ctx>),
    NewTheorem(TheoremId<'ctx>, UnresolvedProof<'ctx>),
    NewGrammarCat(CustomGrammarCatId<'ctx>),
    NewGrammarRule(CustomGrammarRuleId<'ctx>),
}

pub fn elaborate_command_decl<'ctx>(
    command_decl: ParseTreeId<'ctx>,
    cmd: CommandId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, (ElaborateAction<'ctx>, Vec<Attribute<'ctx>>)> {
    // command_decl ::= (command_decl) maybe_attribute_anno command

    match_rule! { (ctx, command_decl) =>
        command_decl ::= [maybe_attr_anno, command] => {
            let attrs = elaborate_maybe_attribute_anno(maybe_attr_anno.as_node().unwrap(), ctx)?;
            let cmd = elaborate_command(command.as_node().unwrap(), cmd, scope, ctx)?;
            Ok((cmd, attrs))
        }
    }
}

fn elaborate_maybe_attribute_anno<'ctx>(
    maybe_anno: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<Attribute<'ctx>>> {
    // maybe_attribute_anno ::= (attribute_anno_some) attribute_anno
    //                        | (attribute_anno_none)

    match_rule! { (ctx, maybe_anno) =>
        attribute_anno_none ::= [] => {
            Ok(Vec::new())
        },
        attribute_anno_some ::= [anno] => {
            elaborate_attribute_anno(anno.as_node().unwrap(), ctx)
        }
    }
}

fn elaborate_attribute_anno<'ctx>(
    anno: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<Attribute<'ctx>>> {
    // attribute_anno ::= (attribute_anno) "@" "[" attributes "]"

    match_rule! { (ctx, anno) =>
        attribute_anno ::= [_at, _lbracket, attrs, _rbracket] => {
            elaborate_attributes(attrs.as_node().unwrap(), ctx)
        }
    }
}

fn elaborate_attributes<'ctx>(
    mut attrs: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<Attribute<'ctx>>> {
    // attributes ::= (attributes_one)  attribute
    //              | (attributes_many) attribute "," attributes

    let mut result = Vec::new();

    loop {
        match_rule! { (ctx, attrs) =>
            attributes_one ::= [attr] => {
                let inst = elaborate_custom_grammar(attr.as_node().unwrap(), ctx)?;
                result.push(Attribute(inst));
                break;
            },
            attributes_many ::= [attr, _comma, rest] => {
                let inst = elaborate_custom_grammar(attr.as_node().unwrap(), ctx)?;
                result.push(Attribute(inst));
                attrs = rest.as_node().unwrap();
            }
        }
    }

    Ok(result)
}

pub fn elaborate_command<'ctx>(
    command: ParseTreeId<'ctx>,
    cmd: CommandId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, ElaborateAction<'ctx>> {
    // command ::= (module_command)           module_command
    //           | (syntax_cat_command)       syntax_cat_command
    //           | (syntax_command)           syntax_command
    //           | (notation_command)         notation_command
    //           | (definition_command)       definition_command
    //           | (axiom_command)            axiom_command
    //           | (theorem_command)          theorem_command
    //           | (grammar_category_command) grammar_category_command
    //           | (tactic_command)           tactic_command
    //           | (attribute_command)        attribute_command

    match_rule! { (ctx, command) =>
        module_command ::= [module_cmd] => {
            let new_source = elaborate_module(module_cmd.as_node().unwrap(), ctx)?;
            Ok(ElaborateAction::NewSource(new_source))
        },
        syntax_cat_command ::= [cat_cmd] => {
            let cat = elaborate_syntax_cat(cat_cmd.as_node().unwrap(), ctx)?;
            Ok(ElaborateAction::NewFormalCat(cat))
        },
        syntax_command ::= [syntax_cmd] => {
            let rule = elaborate_syntax(syntax_cmd.as_node().unwrap(), ctx)?;
            Ok(ElaborateAction::NewFormalRule(rule))
        },
        notation_command ::= [notation_cmd] => {
            let notation = elaborate_notation(notation_cmd.as_node().unwrap(), ctx)?;
            Ok(ElaborateAction::NewNotation(notation))
        },
        definition_command ::= [definition_cmd] => {
            let new_scope = elaborate_definition(definition_cmd.as_node().unwrap(), cmd, scope, ctx)?;
            Ok(ElaborateAction::NewDefinition(new_scope))
        },
        axiom_command ::= [axiom_cmd] => {
            let (thm_id, proof) = elaborate_axiom(axiom_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewTheorem(thm_id, proof))
        },
        theorem_command ::= [theorem_cmd] => {
            let (thm_id, proof) = elaborate_theorem(theorem_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewTheorem(thm_id, proof))
        },
        grammar_category_command ::= [grammar_cat_cmd] => {
            let cat = elaborate_grammar_category(grammar_cat_cmd.as_node().unwrap(), ctx)?;
            Ok(ElaborateAction::NewGrammarCat(cat))
        },
        tactic_command ::= [tactic_cmd] => {
            let rule = elaborate_tactic_def(tactic_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewGrammarRule(rule))
        },
        attribute_command ::= [attribute_cmd] => {
            let rule = elaborate_attribute_def(attribute_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewGrammarRule(rule))
        },
    }
}

fn elaborate_module<'ctx>(module: ParseTreeId<'ctx>, ctx: &Ctx<'ctx>) -> WResult<'ctx, SourceId> {
    // module_command ::= (module) kw"module" name

    match_rule! { (ctx, module) =>
        module ::= [module_kw, source_id_name] => {
            debug_assert!(module_kw.is_kw(*strings::MODULE));
            let source_id_str = elaborate_name(source_id_name.as_node().unwrap(), ctx)?;
            let source_id = SourceId::new(source_id_str);

            if ctx.sources.has_source(source_id) {
                return Diagnostic::err_module_redeclaration(
                    source_id,
                    source_id_name.span(),
                    ctx.sources.get_decl(source_id),
                );
            }

            let path = source_id_to_path(source_id, ctx.config.math_dir());
            let Ok(text) = std::fs::read_to_string(&path) else {
                return Diagnostic::err_non_existent_file(&path, source_id_name.span());
            };

            ctx.sources
                .add(source_id, text, SourceDecl::Module(source_id_name.span()));

            Ok(source_id)
        }

    }
}

fn elaborate_syntax_cat<'ctx>(
    cat: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, FormalSyntaxCatId<'ctx>> {
    // syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name

    match_rule! { (ctx, cat) =>
        syntax_cat ::= [syntax_kw, cat_name] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX_CAT));
            let cat_name = elaborate_name(cat_name.as_node().unwrap(), ctx)?;

            if ctx.arenas.formal_cats.get(cat_name).is_some() {
                return Diagnostic::err_duplicate_formal_syntax_cat();
            }

            let formal_cat = FormalSyntaxCat::new(cat_name);
            let formal_cat = ctx.arenas.formal_cats.alloc(cat_name, formal_cat);
            Ok(formal_cat)
        }
    }
}

fn elaborate_syntax<'ctx>(
    syntax: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, FormalSyntaxRuleId<'ctx>> {
    // syntax_command ::= (syntax) kw"syntax" name name prec_assoc "::=" syntax_pat_list kw"end"

    match_rule! { (ctx, syntax) =>
        syntax ::= [syntax_kw, rule_name_node, cat, prec_assoc, bnf_replace, pat_list, end_kw] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name_node.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let (prec, assoc) = elaborate_prec_assoc(prec_assoc.as_node().unwrap(), ctx)?;
            let mut pat = elaborate_syntax_pat(pat_list.as_node().unwrap(), ctx)?;
            pat.set_prec(prec);
            pat.set_assoc(assoc);

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat.span());
            };

            if ctx.arenas.formal_rules.get(rule_name).is_some() {
                return Diagnostic::err_duplicate_formal_syntax_rule();
            }

            let rule = FormalSyntaxRule::new(rule_name, cat, pat, rule_name_node.span());
            let rule_id = ctx.arenas.formal_rules.alloc(rule_name, rule);

            Ok(rule_id)
        }
    }
}

fn elaborate_prec_assoc<'ctx>(
    prec_assoc: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, (Precedence, Associativity)> {
    // prec_assoc ::= (prec_assoc_none)
    //              | (prec_assoc_some) "(" maybe_prec maybe_assoc ")"

    match_rule! { (ctx, prec_assoc) =>
        prec_assoc_none ::= [] => Ok((Precedence::default(), Associativity::default())),
        prec_assoc_some ::= [l_paren, prec, assoc, r_paren] => {
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let prec = elaborate_maybe_prec(prec.as_node().unwrap(), ctx)?;
            let assoc = elaborate_maybe_assoc(assoc.as_node().unwrap(), ctx)?;

            Ok((prec, assoc))
        }
    }
}

fn elaborate_maybe_prec<'ctx>(
    maybe_prec: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Precedence> {
    // maybe_prec ::= (prec_none)
    //              | (prec_some) number

    match_rule! { (ctx, maybe_prec) =>
        prec_none ::= [] => Ok(Precedence::default()),
        prec_some ::= [level] => {
            let level = level.as_num().unwrap();
            Ok(Precedence(level))
        }
    }
}

fn elaborate_maybe_assoc<'ctx>(
    maybe_assoc: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Associativity> {
    // maybe_assoc ::= (assoc_none)
    //               | (assoc_left)  "<"
    //               | (assoc_right) ">"

    match_rule! { (ctx, maybe_assoc) =>
        assoc_none  ::= [] => Ok(Associativity::NonAssoc),
        assoc_left  ::= [_l_arrow] => Ok(Associativity::Left),
        assoc_right ::= [_r_arrow] => Ok(Associativity::Right)
    }
}

fn elaborate_syntax_pat<'ctx>(
    mut pat_list: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, FormalSyntaxPat<'ctx>> {
    // syntax_pat ::= (syntax_pat_one)  syntax_pat_part
    //              | (syntax_pat_many) syntax_pat_part syntax_pat

    let mut parts = Vec::new();

    loop {
        match_rule! { (ctx, pat_list) =>
            syntax_pat_one ::= [pat] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_syntax_pat_part(pat, ctx)?);
                break;
            },
            syntax_pat_many ::= [pat, rest] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_syntax_pat_part(pat, ctx)?);
                pat_list = rest.as_node().unwrap();
            }
        }
    }

    let pat = FormalSyntaxPat::new(parts);
    Ok(pat)
}

fn elaborate_syntax_pat_part<'ctx>(
    pat: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, FormalSyntaxPatPart<'ctx>> {
    // syntax_pat_part ::= (syntax_pat_cat)     name
    //                   | (syntax_pat_binding) "@" kw"binding" "(" name ")"
    //                   | (syntax_pat_lit)     str

    match_rule! { (ctx, pat) =>
        syntax_pat_part_cat ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(FormalSyntaxPatPart::Cat(cat))
        },
        syntax_pat_part_binding ::= [at, binding_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(binding_kw.is_kw(*strings::BINDING));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(FormalSyntaxPatPart::Binding(cat))
        },
        syntax_pat_part_lit ::= [lit] => {
            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(FormalSyntaxPatPart::Lit(lit))
        }
    }
}

fn elaborate_notation<'ctx>(
    notation: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, NotationPatternId<'ctx>> {
    // notation_command ::= (notation) kw"notation" name name prec_assoc "::=" notation_pat kw"end"

    match_rule! { (ctx, notation) =>
        notation ::= [notation_kw, rule_name_node, cat, prec_assoc, bnf_replace, pat_list, end_kw] => {
            debug_assert!(notation_kw.is_kw(*strings::NOTATION));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name_node.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let (prec, assoc) = elaborate_prec_assoc(prec_assoc.as_node().unwrap(), ctx)?;
            let pat = elaborate_notation_pat(pat_list.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat.span());
            };

            let pat = NotationPattern::new(
                rule_name,
                cat,
                pat,
                prec,
                assoc,
                NotationPatternSource::UserDeclared(rule_name_node.span()),
            );
            Ok(ctx.arenas.notations.alloc(pat))
        }
    }
}

fn elaborate_notation_pat<'ctx>(
    mut pat_list: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<NotationPatternPart<'ctx>>> {
    // notation_pat ::= (notation_pat_one)  notation_pat_part
    //                | (notation_pat_many) notation_pat_part notation_pat

    let mut parts = Vec::new();

    loop {
        match_rule! { (ctx, pat_list) =>
            notation_pat_one ::= [pat] => {
                parts.push(pat.as_node().unwrap());
                break;
            },
            notation_pat_many ::= [pat, rest] => {
                parts.push(pat.as_node().unwrap());
                pat_list = rest.as_node().unwrap();
            }
        }
    }

    // Extract the list of bindings
    let mut bindings = FxHashMap::default();
    for &part in &parts {
        let Some((name, cat)) = elaborate_notation_pat_binding(part, ctx)? else {
            continue;
        };

        if bindings.insert(name, (bindings.len(), cat)).is_some() {
            return Diagnostic::err_duplicate_pattern_binding(name, part.span());
        }
    }

    let mut elaborated_parts = Vec::new();
    for &part in &parts {
        elaborated_parts.push(elaborate_notation_pat_part(part, &bindings, ctx)?);
    }

    Ok(elaborated_parts)
}

fn elaborate_notation_pat_binding<'ctx>(
    pat: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Option<(Ustr, FormalSyntaxCatId<'ctx>)>> {
    // notation_pat ::= (notation_pat_lit)     str
    //                | (notation_pat_kw)      "@" kw"kw" str
    //                | (notation_pat_name)    "@" kw"name"
    //                | (notation_pat_cat)     name maybe_notation_pat_term_args
    //                | (notation_pat_binding) name ":" "@" kw"binding" "(" name ")"

    match_rule! { (ctx, pat) =>
        notation_pat_binding ::= [name_node, colon, at, binding_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(binding_kw.is_kw(*strings::BINDING));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let name = elaborate_name(name_node.as_node().unwrap(), ctx)?;

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(Some((name, cat)))
        },
        notation_pat_lit ::= [_lit] => {
            Ok(None)
        },
        notation_pat_kw ::= [_at, _kw_kw, _lit] => {
            Ok(None)
        },
        notation_pat_name ::= [_at, _name_kw] => {
            Ok(None)
        },
        notation_pat_cat ::= [_cat_name_node, _maybe_args] => {
            Ok(None)
        },
    }
}

fn elaborate_notation_pat_part<'ctx>(
    pat: ParseTreeId<'ctx>,
    bindings: &FxHashMap<Ustr, (usize, FormalSyntaxCatId<'ctx>)>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, NotationPatternPart<'ctx>> {
    // notation_pat ::= (notation_pat_lit)     str
    //                | (notation_pat_kw)      "@" kw"kw" str
    //                | (notation_pat_name)    "@" kw"name"
    //                | (notation_pat_cat)     name maybe_notation_pat_term_args
    //                | (notation_pat_binding) @name ":" "@" kw"binding" "(" name ")"

    match_rule! { (ctx, pat) =>
        notation_pat_lit ::= [lit] => {
            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(NotationPatternPart::Lit(lit))
        },
        notation_pat_kw ::= [at, kw_kw, lit] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(kw_kw.is_kw(*strings::KW));

            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(NotationPatternPart::Kw(lit))
        },
        notation_pat_name ::= [at, name_kw] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(name_kw.is_kw(*strings::NAME));

            Ok(NotationPatternPart::Name)
        },
        notation_pat_cat ::= [cat_name_node, maybe_args] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            let args = elaborate_maybe_notation_pat_args(maybe_args.as_node().unwrap(), bindings, ctx)?;

            let cat_part = NotationPatternPartCat::new(cat, args);
            Ok(NotationPatternPart::Cat(cat_part))
        },
        notation_pat_binding ::= [_name, colon, at, binding_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(binding_kw.is_kw(*strings::BINDING));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let cat = ctx.arenas.formal_cats.get(cat_name).expect("checked previously");

            Ok(NotationPatternPart::Binding(cat))
        }
    }
}

fn elaborate_maybe_notation_pat_args<'ctx>(
    maybe_args: ParseTreeId<'ctx>,
    bindings: &FxHashMap<Ustr, (usize, FormalSyntaxCatId<'ctx>)>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<(usize, FormalSyntaxCatId<'ctx>)>> {
    // maybe_notation_pat_term_args ::= (maybe_notation_pat_term_args_none)
    //                                | (maybe_notation_pat_term_args_some) "(" notation_pat_term_args ")"

    match_rule! { (ctx, maybe_args) =>
        maybe_notation_pat_term_args_none ::= [] => {
            Ok(Vec::new())
        },
        maybe_notation_pat_term_args_some ::= [l_paren, args, r_paren] => {
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));
            elaborate_notation_pat_args(args.as_node().unwrap(), bindings, ctx)
        }
    }
}

fn elaborate_notation_pat_args<'ctx>(
    mut args: ParseTreeId<'ctx>,
    bindings: &FxHashMap<Ustr, (usize, FormalSyntaxCatId<'ctx>)>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<(usize, FormalSyntaxCatId<'ctx>)>> {
    // notation_pat_term_args ::= (notation_pat_term_args_one)  @name
    //                          | (notation_pat_term_args_many) @name notation_pat_term_args

    let mut elaborated_args = Vec::new();

    loop {
        let (name_node, rest) = match_rule! { (ctx, args) =>
            notation_pat_term_args_one ::= [name] => {
                (name.as_node().unwrap(), None)
            },
            notation_pat_term_args_many ::= [name, rest] => {
                (name.as_node().unwrap(), Some(rest.as_node().unwrap()))
            }
        };

        let name = elaborate_name(name_node, ctx)?;
        match bindings.get(&name) {
            Some(arg) => elaborated_args.push(*arg),
            None => return Diagnostic::err_unknown_pattern_binding(name, name_node.span()),
        }

        match rest {
            Some(rest) => args = rest,
            None => break,
        }
    }

    Ok(elaborated_args)
}

fn elaborate_grammar_category<'ctx>(
    cat: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarCatId<'ctx>> {
    // grammar_category_command ::= (grammar_category) kw"tactic_category" name

    match_rule! { (ctx, cat) =>
        grammar_category ::= [grammar_category_kw, cat_name] => {
            debug_assert!(grammar_category_kw.is_kw(*strings::GRAMMAR_CATEGORY));
            let cat_name_node = cat_name.as_node().unwrap();
            let cat_name = elaborate_name(cat_name_node, ctx)?;

            if ctx.arenas.grammar_cats.get(cat_name).is_some() {
                return Diagnostic::err_duplicate_grammar_cat();
            }

            // Check if the category name conflicts with reserved Luau types
            // (tactic names are converted to PascalCase for Lua)
            let lua_name = crate::util::name_to_lua(cat_name.as_str());
            if RESERVED_LUAU_TYPES.contains(&lua_name.as_str()) {
                return Diagnostic::err_reserved_grammar_cat_name(cat_name, cat_name_node.span());
            }

            let grammar_cat = CustomGrammarCat::new(cat_name);
            let grammar_cat = ctx.arenas.grammar_cats.alloc(cat_name, grammar_cat);
            Ok(grammar_cat)
        }
    }
}

fn elaborate_tactic_def<'ctx>(
    tactic: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarRuleId<'ctx>> {
    // tactic_command ::= (tactic) kw"tactic" name name prec_assoc "::=" tactic_pat kw"end"

    match_rule! { (ctx, tactic) =>
        tactic ::= [tactic_kw, rule_name, cat, prec_assoc, bnf_replace, pat_list, end_kw] => {
            debug_assert!(tactic_kw.is_kw(*strings::TACTIC));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let (prec, assoc) = elaborate_prec_assoc(prec_assoc.as_node().unwrap(), ctx)?;
            let pat = elaborate_grammar_pat(pat_list.as_node().unwrap(), prec, assoc, ctx)?;

            let Some(cat) = ctx.arenas.grammar_cats.get(cat_name) else {
                return Diagnostic::err_unknown_grammar_cat(cat_name, cat.span());
            };

            if ctx.arenas.grammar_rules.get(rule_name).is_some() {
                return Diagnostic::err_duplicate_grammar_rule();
            }

            let scope = ctx.scopes.alloc(scope.clone());
            let rule = CustomGrammarRule::new(rule_name, cat, pat, scope);
            let rule_id = ctx.arenas.grammar_rules.alloc(rule_name, rule);

            Ok(rule_id)
        }
    }
}

fn elaborate_attribute_def<'ctx>(
    attribute: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarRuleId<'ctx>> {
    // tactic_command ::= (tactic) kw"tactic" name name prec_assoc "::=" tactic_pat kw"end"

    match_rule! { (ctx, attribute) =>
        attribute ::= [attribute_kw, rule_name, cat, prec_assoc, bnf_replace, pat_list, end_kw] => {
            debug_assert!(attribute_kw.is_kw(*strings::ATTRIBUTE));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let (prec, assoc) = elaborate_prec_assoc(prec_assoc.as_node().unwrap(), ctx)?;
            let pat = elaborate_grammar_pat(pat_list.as_node().unwrap(), prec, assoc, ctx)?;

            let Some(cat) = ctx.arenas.grammar_cats.get(cat_name) else {
                return Diagnostic::err_unknown_grammar_cat(cat_name, cat.span());
            };

            if ctx.arenas.grammar_rules.get(rule_name).is_some() {
                return Diagnostic::err_duplicate_grammar_rule();
            }

            let scope = ctx.scopes.alloc(scope.clone());
            let rule = CustomGrammarRule::new(rule_name, cat, pat, scope);
            let rule_id = ctx.arenas.grammar_rules.alloc(rule_name, rule);

            Ok(rule_id)
        }
    }
}

fn elaborate_grammar_pat<'ctx>(
    mut pat_list: ParseTreeId<'ctx>,
    prec: Precedence,
    assoc: Associativity,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarPat<'ctx>> {
    // grammar_pat ::= (grammar_pat_none)
    //               | (grammar_pat_many) grammar_pat_part grammar_pat

    let mut parts = Vec::new();

    loop {
        match_rule! { (ctx, pat_list) =>
            grammar_pat_none ::= [] => {
                break;
            },
            grammar_pat_many ::= [pat, rest] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_grammar_pat_part(pat, ctx)?);
                pat_list = rest.as_node().unwrap();
            }
        }
    }

    Ok(CustomGrammarPat::new(parts, prec, assoc))
}

fn elaborate_grammar_pat_part<'ctx>(
    pat: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarPatPart<'ctx>> {
    // grammar_pat_part ::= (grammar_pat_part) maybe_label grammar_pat_part_core

    match_rule! { (ctx, pat) =>
        grammar_pat_part ::= [maybe_label, core] => {
            let label = elaborate_maybe_label(maybe_label.as_node().unwrap(), ctx)?;
            let core = elaborate_grammar_pat_part_core(core.as_node().unwrap(), ctx)?;
            Ok(CustomGrammarPatPart::new(label, core))
        }
    }
}

fn elaborate_maybe_label<'ctx>(
    maybe_label: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Option<Ustr>> {
    // maybe_label ::= (label_none)
    //               | (label_some) name ":"

    match_rule! { (ctx, maybe_label) =>
        label_none ::= [] => Ok(None),
        label_some ::= [label_node, colon] => {
            debug_assert!(colon.is_lit(*strings::COLON));
            let label = elaborate_name(label_node.as_node().unwrap(), ctx)?;

            // Check for reserved label names
            if label == *strings::RESERVED_RULE || label == *strings::RESERVED_SPAN {
                _ = Diagnostic::err_reserved_grammar_label::<()>(label, label_node.span());
                return Ok(None);
            }

            Ok(Some(label))
        }
    }
}

fn elaborate_grammar_pat_part_core<'ctx>(
    core: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarPatPartCore<'ctx>> {
    // grammar_pat_part_core ::= (core_lit)          str
    //                         | (core_kw)           "@" kw"kw" str
    //                         | (core_name)         "@" kw"name"
    //                         | (core_cat)          name
    //                         | (core_fragment)     "@" kw"fragment" "(" name ")"
    //                         | (core_any_fragment) "@" kw"any_fragment"
    //                         | (core_fact)         "@" kw"fact"

    match_rule! { (ctx, core) =>
        core_lit ::= [lit] => {
            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(CustomGrammarPatPartCore::Lit(lit))
        },
        core_kw ::= [at, kw_kw, lit] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(kw_kw.is_kw(*strings::KW));

            let lit = elaborate_str_lit(lit.as_node().unwrap(), ctx)?;
            Ok(CustomGrammarPatPartCore::Kw(lit))
        },
        core_name ::= [at, name_kw] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(name_kw.is_kw(*strings::NAME));

            Ok(CustomGrammarPatPartCore::Name)
        },
        core_cat ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.grammar_cats.get(cat_name) else {
                return Diagnostic::err_unknown_grammar_cat(cat_name, cat_name_node.span());
            };

            Ok(CustomGrammarPatPartCore::Cat(cat))
        },
        core_fragment ::= [at, fragment_kw, lparen, cat_name_node, rparen] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(fragment_kw.is_kw(*strings::FRAGMENT));
            debug_assert!(lparen.is_lit(*strings::LEFT_PAREN));
            debug_assert!(rparen.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.parse_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(CustomGrammarPatPartCore::Frag(cat))
        },
        core_any_fragment ::= [at, any_fragment_kw] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(any_fragment_kw.is_kw(*strings::ANY_FRAGMENT));

            Ok(CustomGrammarPatPartCore::AnyFrag)
        },
        core_fact ::= [at, fact_kw] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(fact_kw.is_kw(*strings::FACT));

            Ok(CustomGrammarPatPartCore::Fact)
        }
    }
}

fn elaborate_definition<'ctx>(
    definition: ParseTreeId<'ctx>,
    cmd: CommandId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Scope<'ctx>> {
    // definition_command ::= (definition) kw"definition" notation_binding ":=" fragment kw"end"

    match_rule! { (ctx, definition) =>
        definition ::= [definition_kw, notation_binding, assign, fragment_node, end_kw] => {
            debug_assert!(definition_kw.is_kw(*strings::DEFINITION));
            debug_assert!(assign.is_lit(*strings::ASSIGN));
            debug_assert!(end_kw.is_kw(*strings::END));

            let binding_possibilities = elaborate_notation_binding(notation_binding.as_node().unwrap(), None, None, ctx)?;
            let possible_frag_cats = elaborate_any_fragment(fragment_node.as_node().unwrap());

            let mut best_prec = None;
            let mut solutions = Vec::new();
            let mut parse_errors = Vec::new();

            for possibility in binding_possibilities {
                let Some(frag) = possible_frag_cats.get(&possibility.binding.pattern().cat()) else {
                    continue;
                };

                let mut scope = scope.clone();
                for (i, hole_binding) in possibility.holes.iter().enumerate() {
                    let hole_cat = hole_binding.pattern().cat();
                    let entry = ScopeEntry::new_hole(hole_cat, i, DefinitionSource::DefinitionHole);
                    scope = scope.child_with(*hole_binding, entry);
                }

                // Try parsing the fragment given these possibilities.
                let parse = match parse_fragment(UnresolvedFrag(*frag), &scope, ctx)? {
                    Ok(parse) => parse,
                    Err(err) => {
                        parse_errors.push((possibility.binding, err));
                        continue
                    },
                };

                let this_prec = possibility.binding.pattern().prec();
                if best_prec.is_none_or(|best_prec| this_prec > best_prec)  {
                    best_prec = Some(this_prec);
                    solutions.clear();
                    parse_errors.clear();
                }

                solutions.push((possibility.binding, parse));
            }

            match solutions.as_slice() {
                [] => {
                    let mut diags = Vec::new();
                    for (binding, err) in parse_errors {
                        let diag = Diagnostic::err_frag_parse_failure(fragment_node.span(), err)
                            .with_info(
                                &format!("assuming the intended notation was `{}`", binding.print()),
                                vec![DiagnosticSpan::new_info("", notation_binding.span())]
                            );
                        diags.push(diag);
                    }
                    Err(diags)
                }
                [(binding, frag)] => {
                    let entry = ScopeEntry::new(*frag, DefinitionSource::DefinitionCmd(cmd));
                    Ok(scope.child_with(*binding, entry))
                },
                [..] => Diagnostic::err_multiple_notations(notation_binding.span(), solutions.iter().map(|s| s.0).collect()),
            }
        }
    }
}

fn elaborate_any_fragment<'ctx>(
    any_frag: ParseTreeId<'ctx>,
) -> FxHashMap<FormalSyntaxCatId<'ctx>, ParseTreeId<'ctx>> {
    any_frag
        .possibilities()
        .iter()
        .map(|possibility| {
            let frag = possibility.children()[0];
            let frag = frag.as_node().unwrap();
            let SyntaxCategorySource::FormalLang(formal_cat) = frag.cat().source() else {
                unreachable!();
            };

            (formal_cat, frag)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct BindingResolution<'ctx> {
    pub binding: NotationBindingId<'ctx>,
    pub holes: Vec<NotationBindingId<'ctx>>,
}

fn elaborate_notation_binding<'ctx>(
    notation_binding: ParseTreeId<'ctx>,
    expected_cat: Option<FormalSyntaxCatId<'ctx>>,
    expected_holes: Option<&[NotationSignatureHole<'ctx>]>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<BindingResolution<'ctx>>> {
    fn children_to_binding<'ctx>(
        resolved: &mut Vec<BindingResolution<'ctx>>,
        children: &ParseTreeChildren<'ctx>,
        pattern: NotationPatternId<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> WResult<'ctx, ()> {
        let mut holes = pattern.signature().holes().iter();

        let mut name_instantiations = Vec::new();
        let mut sub_pattern_instantiations = Vec::new();

        for (part, child) in pattern.parts().iter().zip(children.children()) {
            match part {
                NotationPatternPart::Name => {
                    let name = elaborate_name(child.as_node().unwrap(), ctx)?;
                    name_instantiations.push(name);
                }
                NotationPatternPart::Cat(_) => {
                    let hole = holes.next().unwrap();
                    let args = hole
                        .args()
                        .iter()
                        .map(|&a| NotationSignatureHole::new(a, Vec::new()))
                        .collect_vec();
                    let resolved = elaborate_notation_binding(
                        child.as_node().unwrap(),
                        Some(hole.cat()),
                        Some(&args),
                        ctx,
                    )?;

                    // The resolutions have no args so we can throw those away.
                    let resolved = resolved.iter().map(|r| r.binding).collect_vec();

                    sub_pattern_instantiations.push(resolved);
                }
                NotationPatternPart::Binding(_)
                | NotationPatternPart::Lit(_)
                | NotationPatternPart::Kw(_) => {}
            }
        }

        // Get every set of possibilities for ambiguous sub patterns
        let product = sub_pattern_instantiations
            .iter()
            .map(|p| p.iter().cloned())
            .multi_cartesian_product()
            .collect_vec();

        for holes in product {
            let binding = NotationBinding::new(pattern, name_instantiations.clone());
            let binding = ctx.arenas.notation_bindings.intern(binding);
            let resolution = BindingResolution { binding, holes };
            resolved.push(resolution);
        }

        Ok(())
    }

    let mut resolved = Vec::new();

    for possibility in notation_binding.possibilities() {
        let rule = possibility.rule();
        let &ParseRuleSource::Notation(notation) = rule.source() else {
            unreachable!();
        };

        // If we are looking for a specific cat and this isn't it, continue.
        if expected_cat
            .map(|cat| cat != notation.signature().cat())
            .unwrap_or(false)
        {
            continue;
        }

        // If we are looking for a specific set of holes and this isn't it, continue.
        if expected_holes
            .map(|holes| holes != notation.signature().holes())
            .unwrap_or(false)
        {
            continue;
        }

        children_to_binding(&mut resolved, possibility, notation, ctx)?;
    }

    Ok(resolved)
}

fn parse_hypotheses_and_conclusion<'ctx>(
    un_hypotheses: Vec<UnresolvedFact<'ctx>>,
    un_conclusion: UnresolvedFrag<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, (Vec<PresFact<'ctx>>, PresFrag<'ctx>)> {
    let mut errs = Vec::new();

    let mut hypotheses = Vec::new();
    for un_hypothesis in un_hypotheses {
        let assumption = match un_hypothesis.assumption {
            Some(assumption) => match parse_fragment(assumption, scope, ctx)? {
                Ok(assumption) => Some(assumption),
                Err(err) => {
                    errs.push(Diagnostic::err_frag_parse_failure(assumption.0.span(), err));
                    continue;
                }
            },
            None => None,
        };
        let conclusion = match parse_fragment(un_hypothesis.conclusion, scope, ctx)? {
            Ok(conclusion) => conclusion,
            Err(err) => {
                errs.push(Diagnostic::err_frag_parse_failure(
                    un_hypothesis.conclusion.0.span(),
                    err,
                ));
                continue;
            }
        };
        hypotheses.push(PresFact::new(assumption, conclusion));
    }

    let conclusion = match parse_fragment(un_conclusion, scope, ctx)? {
        Ok(conclusion) => Some(conclusion),
        Err(err) => {
            errs.push(Diagnostic::err_frag_parse_failure(
                un_conclusion.0.span(),
                err,
            ));
            None
        }
    };

    if errs.is_empty() {
        Ok((hypotheses, conclusion.unwrap()))
    } else {
        Err(errs)
    }
}

fn elaborate_axiom<'ctx>(
    axiom: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, (TheoremId<'ctx>, UnresolvedProof<'ctx>)> {
    // axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"

    match_rule! { (ctx, axiom) =>
        axiom ::= [axiom_kw, name_node, templates, colon, hypotheses, turnstile, conclusion, end_kw] => {
            debug_assert!(axiom_kw.is_kw(*strings::AXIOM));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let name = elaborate_name(name_node.as_node().unwrap(), ctx)?;
            let templates = elaborate_templates(templates.as_node().unwrap(), ctx)?;

            let my_scope = add_templates_to_scope(&templates, scope, ctx);

            let hypotheses = elaborate_hypotheses(hypotheses.as_node().unwrap(), ctx)?;
            let conclusion = UnresolvedFrag(conclusion.as_node().unwrap());

            let (hypotheses, conclusion) = parse_hypotheses_and_conclusion(hypotheses, conclusion, &my_scope, ctx)?;

            let scope_id = ctx.scopes.alloc(my_scope);

            let theorem_stmt = TheoremStatement::new(name, templates, hypotheses, conclusion, scope_id);
            let theorem_stmt = ctx.arenas.theorem_stmts.alloc(name, theorem_stmt);

            Ok((theorem_stmt, UnresolvedProof::Axiom))
        }
    }
}

fn elaborate_theorem<'ctx>(
    theorem: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<'ctx, (TheoremId<'ctx>, UnresolvedProof<'ctx>)> {
    // theorem_command ::= (theorem) kw"theorem" name templates ":" hypotheses "|-" sentence kw"proof" tactic kw"qed"

    match_rule! { (ctx, theorem) =>
        theorem ::= [theorem_kw, name_node, templates, colon, hypotheses, turnstile, conclusion, proof_kw, tactic, qed_kw] => {
            debug_assert!(theorem_kw.is_kw(*strings::THEOREM));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));
            debug_assert!(proof_kw.is_kw(*strings::PROOF));
            debug_assert!(qed_kw.is_kw(*strings::QED));

            let name = elaborate_name(name_node.as_node().unwrap(), ctx)?;
            let templates = elaborate_templates(templates.as_node().unwrap(), ctx)?;

            let my_scope = add_templates_to_scope(&templates, scope, ctx);

            let hypotheses = elaborate_hypotheses(hypotheses.as_node().unwrap(), ctx)?;
            let conclusion = UnresolvedFrag(conclusion.as_node().unwrap());

            let (hypotheses, conclusion) = parse_hypotheses_and_conclusion(hypotheses, conclusion, &my_scope, ctx)?;

            let scope_id = ctx.scopes.alloc(my_scope);

            let theorem_stmt = TheoremStatement::new(name, templates, hypotheses, conclusion, scope_id);
            let theorem_stmt = ctx.arenas.theorem_stmts.alloc(name, theorem_stmt);

            let proof = elaborate_custom_grammar(tactic.as_node().unwrap(), ctx)?;
            let proof = UnresolvedProof::Theorem(proof);

            Ok((theorem_stmt, proof))
        }
    }
}

fn elaborate_templates<'ctx>(
    mut templates: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<Template<'ctx>>> {
    // templates ::= (template_none)
    //             | (template_many) template templates

    let mut seen_templates = FxHashSet::default();
    let mut templates_list = Vec::new();

    loop {
        match_rule! { (ctx, templates) =>
            template_none ::= [] => {
                return Ok(templates_list);
            },
            template_many ::= [template, rest] => {
                let template = template.as_node().unwrap();

                for template in elaborate_template(template, ctx)? {
                    if seen_templates.contains(&template.binding()) {
                        todo!("Error: duplicate template binding.");
                    }

                    seen_templates.insert(template.binding());
                    templates_list.push(template);
                }

                templates = rest.as_node().unwrap();
            }
        }
    }
}

fn elaborate_template<'ctx>(
    template: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<Template<'ctx>>> {
    // template ::= (template) "[" template_bindings ":" name "]"

    match_rule! { (ctx, template) =>
        template ::= [l_brack, names, colon, cat_name_node, r_brack] => {
            debug_assert!(l_brack.is_lit(*strings::LEFT_BRACKET));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(r_brack.is_lit(*strings::RIGHT_BRACKET));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return Diagnostic::err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            let bindings = elaborate_template_bindings(names.as_node().unwrap(), cat, ctx)?;

            Ok(bindings)
        }
    }
}

fn elaborate_template_bindings<'ctx>(
    mut bindings: ParseTreeId<'ctx>,
    cat: FormalSyntaxCatId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<Template<'ctx>>> {
    // template_bindings ::= (template_bindings_none)
    //                     | (template_bindings_many) notation_binding template_bindings

    let mut binding_list = Vec::new();

    loop {
        match_rule! { (ctx, bindings) =>
            template_bindings_none ::= [] => {
                return Ok(binding_list);
            },
            template_bindings_many ::= [binding, rest] => {
                let binding = binding.as_node().unwrap();

                let mut possibilities = elaborate_notation_binding(binding, Some(cat), None, ctx)?;

                if possibilities.is_empty() {
                    return Diagnostic::err_no_matching_notation_binding(cat.name(), binding.span());
                }

                if possibilities.len() > 1 {
                    return Diagnostic::err_ambiguous_notation_binding(
                        cat.name(),
                        &possibilities,
                        binding.span(),
                    );
                }

                let resolution = possibilities.swap_remove(0);
                let template = Template::new(cat, resolution.binding, resolution.holes);

                binding_list.push(template);
                bindings = rest.as_node().unwrap();
            }
        }
    }
}

fn elaborate_hypotheses<'ctx>(
    hypotheses: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Vec<UnresolvedFact<'ctx>>> {
    // hypotheses ::= (hypotheses_none)
    //              | (hypotheses_many) hypothesis hypotheses

    let mut hypotheses_list = Vec::new();
    let mut next_hypotheses = Some(hypotheses);

    while let Some(hypotheses) = next_hypotheses {
        match_rule! { (ctx, hypotheses) =>
            hypotheses_none ::= [] => {
                next_hypotheses = None;
            },
            hypotheses_many ::= [hypothesis, rest] => {
                let hypothesis = hypothesis.as_node().unwrap();
                let rest = rest.as_node().unwrap();

                hypotheses_list.push(elaborate_hypothesis(hypothesis, ctx)?);
                next_hypotheses = Some(rest);
            }
        }
    }

    Ok(hypotheses_list)
}

fn elaborate_hypothesis<'ctx>(
    hypothesis: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, UnresolvedFact<'ctx>> {
    // hypothesis ::= (hypothesis) "(" fact ")"

    match_rule! { (ctx, hypothesis) =>
        hypothesis ::= [l_paren, fact, r_paren] => {
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let fact = fact.as_node().unwrap();
            let fact = elaborate_fact(fact, ctx)?;

            Ok(fact)
        }
    }
}

fn elaborate_fact<'ctx>(
    fact: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, UnresolvedFact<'ctx>> {
    // fact ::= (fact_assumption) kw"assume" sentence "|-" sentence
    //        | (fact_sentence)   sentence

    match_rule! { (ctx, fact) =>
        fact_assumption ::= [assume_kw, assumption, turnstile, conclusion] => {
            debug_assert!(assume_kw.is_kw(*strings::ASSUME));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));

            let assumption = UnresolvedFrag(assumption.as_node().unwrap());
            let conclusion = UnresolvedFrag(conclusion.as_node().unwrap());
            Ok(UnresolvedFact {
                assumption: Some(assumption),
                conclusion,
            })
        },
        fact_sentence ::= [conclusion] => {
            let conclusion = UnresolvedFrag(conclusion.as_node().unwrap());
            Ok(UnresolvedFact {
                assumption: None,
                conclusion,
            })
        }
    }
}

fn elaborate_custom_grammar<'ctx>(
    grammar: ParseTreeId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, CustomGrammarInst<'ctx>> {
    let children = expect_unambiguous(grammar)?;
    let rule = children.rule();
    let grammar_rule = rule.source().get_tactic_rule();

    let mut grammar_children = Vec::new();
    for (part, child) in grammar_rule
        .pattern()
        .parts()
        .iter()
        .zip(children.children().iter())
    {
        let g_child = match part.part() {
            CustomGrammarPatPartCore::Kw(str) => {
                let spanned_str = SpannedStr::new(*str, child.span());
                CustomGrammarInstPart::Kw(spanned_str)
            }
            CustomGrammarPatPartCore::Lit(str) => {
                let spanned_str = SpannedStr::new(*str, child.span());
                CustomGrammarInstPart::Lit(spanned_str)
            }
            CustomGrammarPatPartCore::Name => {
                let name = elaborate_name(child.as_node().unwrap(), ctx)?;
                let spanned_str = SpannedStr::new(name, child.span());
                CustomGrammarInstPart::Name(spanned_str)
            }
            CustomGrammarPatPartCore::Cat(_) => {
                let inst = elaborate_custom_grammar(child.as_node().unwrap(), ctx)?;
                CustomGrammarInstPart::SubInst(inst)
            }
            CustomGrammarPatPartCore::Frag(_) => {
                let frag = UnresolvedFrag(child.as_node().unwrap());
                CustomGrammarInstPart::Frag(frag)
            }
            CustomGrammarPatPartCore::AnyFrag => {
                let frag = UnresolvedAnyFrag(child.as_node().unwrap());
                CustomGrammarInstPart::AnyFrag(frag)
            }
            CustomGrammarPatPartCore::Fact => {
                let fact = elaborate_fact(child.as_node().unwrap(), ctx)?;
                CustomGrammarInstPart::Fact(fact)
            }
        };
        grammar_children.push(g_child);
    }

    Ok(CustomGrammarInst::new(
        grammar_rule,
        grammar.span(),
        grammar_children,
    ))
}

pub fn elaborate_name<'ctx>(name: ParseTreeId<'ctx>, ctx: &Ctx<'ctx>) -> WResult<'ctx, Ustr> {
    match_rule! { (ctx, name) =>
        name ::= [name_atom] => {
            let name = name_atom.as_name().unwrap();
            Ok(name)
        }
    }
}

pub fn elaborate_str_lit<'ctx>(str_lit: ParseTreeId<'ctx>, ctx: &Ctx<'ctx>) -> WResult<'ctx, Ustr> {
    match_rule! { (ctx, str_lit) =>
        str ::= [str_atom] => {
            let str_lit = str_atom.as_str_lit().unwrap();
            Ok(str_lit)
        }
    }
}

fn expect_unambiguous<'ctx>(id: ParseTreeId<'ctx>) -> WResult<'ctx, &'ctx ParseTreeChildren<'ctx>> {
    match id.0.possibilities() {
        [] => unreachable!("No possibilities in parse tree."),
        [possibility] => Ok(possibility),
        _ => Diagnostic::err_ambiguous_parse(id.span()),
    }
}
