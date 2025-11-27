use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        SourceId, Span,
        parse_state::{Associativity, ParseRuleSource, Precedence, SyntaxCategorySource},
        parse_tree::{ParseTreeChildren, ParseTreeId},
        source_cache::{SourceDecl, source_id_to_path},
    },
    semant::{
        formal_syntax::{
            CatMap, FormalSyntaxCat, FormalSyntaxCatId, FormalSyntaxPat, FormalSyntaxPatPart,
            FormalSyntaxRule, FormalSyntaxRuleId,
        },
        fragment::{FragHead, Fragment, FragmentId},
        notation::{
            NotationBinding, NotationBindingId, NotationInstantiationPart, NotationPattern,
            NotationPatternId, NotationPatternPart,
        },
        parse_fragment::{UnresolvedFact, UnresolvedFrag, parse_fragment},
        scope::{Scope, ScopeEntry},
        theorems::{Fact, Template, TheoremId, TheoremStatement, UnresolvedProof},
    },
    strings,
};
use rustc_hash::{FxHashMap, FxHashSet};
use ustr::Ustr;

macro_rules! failed_to_match_builtin {
    ($rule:expr, $ctx:expr) => {
        panic!("Failed to match builtin parse tree: {}", $rule.name());
    };
}

macro_rules! match_rule {
    (($ctx:expr, $tree_id:expr) => $($rule:ident ::= [$($child:ident),*] => $body:expr),+ $(,)?) => {{
        let tree = $tree_id;
        let children = expect_unambiguous(tree, $ctx)?;
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

pub enum ElaborateAction<'ctx> {
    NewSource(SourceId),
    NewFormalCat(FormalSyntaxCatId<'ctx>),
    NewFormalRule(FormalSyntaxRuleId<'ctx>),
    NewNotation(NotationPatternId<'ctx>),
    NewDefinition(Scope<'ctx>),
    NewTheorem(TheoremId<'ctx>),
    None,
}

pub fn elaborate_command<'ctx>(
    command: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<ElaborateAction<'ctx>> {
    // command ::= (module_command)     module_command
    //           | (syntax_cat_command) syntax_cat_command
    //           | (syntax_command)     syntax_command
    //           | (notation_command)   notation_command
    //           | (definition_command) definition_command
    //           | (axiom_command)      axiom_command
    //           | (theorem_command)    theorem_command

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
            let new_scope = elaborate_definition(definition_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewDefinition(new_scope))
        },
        axiom_command ::= [axiom_cmd] => {
            let thm_id = elaborate_axiom(axiom_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewTheorem(thm_id))
        },
        theorem_command ::= [theorem_cmd] => {
            let thm_id = elaborate_theorem(theorem_cmd.as_node().unwrap(), scope, ctx)?;
            Ok(ElaborateAction::NewTheorem(thm_id))
        },
    }
}

fn elaborate_module<'ctx>(module: ParseTreeId<'ctx>, ctx: &mut Ctx<'ctx>) -> WResult<SourceId> {
    // module_command ::= (module) kw"module" name

    match_rule! { (ctx, module) =>
        module ::= [module_kw, source_id_name] => {
            debug_assert!(module_kw.is_kw(*strings::MODULE));
            let source_id_str = elaborate_name(source_id_name.as_node().unwrap(), ctx)?;
            let source_id = SourceId::new(source_id_str);

            if ctx.sources.has_source(source_id) {
                return ctx.diags.err_module_redeclaration(
                    source_id,
                    source_id_name.span(),
                    ctx.sources.get_decl(source_id),
                );
            }

            let path = source_id_to_path(source_id, ctx.sources.root_dir());
            let Ok(text) = std::fs::read_to_string(&path) else {
                return ctx
                    .diags
                    .err_non_existent_file(&path, source_id_name.span());
            };

            ctx.sources
                .add(source_id, text, SourceDecl::Module(source_id_name.span()));

            Ok(source_id)
        }

    }
}

fn elaborate_syntax_cat<'ctx>(
    cat: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<FormalSyntaxCatId<'ctx>> {
    // syntax_cat_command ::= (syntax_cat) kw"syntax_cat" name

    match_rule! { (ctx, cat) =>
        syntax_cat ::= [syntax_kw, cat_name] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX_CAT));
            let cat_name = elaborate_name(cat_name.as_node().unwrap(), ctx)?;

            if ctx.arenas.formal_cats.get(cat_name).is_some() {
                return ctx.diags.err_duplicate_formal_syntax_cat();
            }

            let formal_cat = FormalSyntaxCat::new(cat_name);
            let formal_cat = ctx.arenas.formal_cats.alloc(cat_name, formal_cat);
            Ok(formal_cat)
        }
    }
}

fn elaborate_syntax<'ctx>(
    syntax: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<FormalSyntaxRuleId<'ctx>> {
    // syntax_command ::= (syntax) kw"syntax" name name prec_assoc "::=" syntax_pat_list kw"end"

    match_rule! { (ctx, syntax) =>
        syntax ::= [syntax_kw, rule_name, cat, prec_assoc, bnf_replace, pat_list, end_kw] => {
            debug_assert!(syntax_kw.is_kw(*strings::SYNTAX));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let (prec, assoc) = elaborate_prec_assoc(prec_assoc.as_node().unwrap(), ctx)?;
            let mut pat = elaborate_syntax_pat(pat_list.as_node().unwrap(), ctx)?;
            pat.set_prec(prec);
            pat.set_assoc(assoc);

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat.span());
            };

            if ctx.arenas.formal_rules.get(rule_name).is_some() {
                return ctx.diags.err_duplicate_formal_syntax_rule();
            }

            let rule = FormalSyntaxRule::new(rule_name, cat, pat);
            let rule_id = ctx.arenas.formal_rules.alloc(rule_name, rule);

            Ok(rule_id)
        }
    }
}

fn elaborate_prec_assoc<'ctx>(
    prec_assoc: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(Precedence, Associativity)> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<Precedence> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<Associativity> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<FormalSyntaxPat<'ctx>> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<FormalSyntaxPatPart<'ctx>> {
    // syntax_pat_part ::= (syntax_pat_cat)     name
    //                   | (syntax_pat_binding) "@" kw"binding" "(" name ")"
    //                   | (syntax_pat_lit)     str

    match_rule! { (ctx, pat) =>
        syntax_pat_part_cat ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
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
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<NotationPatternId<'ctx>> {
    // notation_command ::= (notation) kw"notation" name name prec_assoc "::=" notation_pat kw"end"

    match_rule! { (ctx, notation) =>
        notation ::= [notation_kw, rule_name, cat, prec_assoc, bnf_replace, pat_list, end_kw] => {
            debug_assert!(notation_kw.is_kw(*strings::NOTATION));
            debug_assert!(bnf_replace.is_lit(*strings::BNF_REPLACE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let rule_name = elaborate_name(rule_name.as_node().unwrap(), ctx)?;
            let cat_name = elaborate_name(cat.as_node().unwrap(), ctx)?;
            let (prec, assoc) = elaborate_prec_assoc(prec_assoc.as_node().unwrap(), ctx)?;
            let pat = elaborate_notation_pat(pat_list.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat.span());
            };

            let pat = NotationPattern::new(rule_name, cat, pat, prec, assoc);
            Ok(ctx.arenas.notations.alloc(pat))
        }
    }
}

fn elaborate_notation_pat<'ctx>(
    mut pat_list: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Vec<NotationPatternPart<'ctx>>> {
    // notation_pat ::= (notation_pat_one)  notation_pat_part
    //                | (notation_pat_many) notation_pat_part notation_pat

    let mut parts = Vec::new();

    loop {
        match_rule! { (ctx, pat_list) =>
            notation_pat_one ::= [pat] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_notation_pat_part(pat, ctx)?);
                break;
            },
            notation_pat_many ::= [pat, rest] => {
                let pat = pat.as_node().unwrap();
                parts.push(elaborate_notation_pat_part(pat, ctx)?);
                pat_list = rest.as_node().unwrap();
            }
        }
    }

    Ok(parts)
}

fn elaborate_notation_pat_part<'ctx>(
    pat: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<NotationPatternPart<'ctx>> {
    // notation_pat ::= (notation_pat_lit)     str
    //                | (notation_pat_kw)      "@" kw"kw" str
    //                | (notation_pat_name)    "@" kw"name"
    //                | (notation_pat_cat)     name
    //                | (notation_pat_binding) "@" kw"binding" "(" name ")"

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
        notation_pat_cat ::= [cat_name_node] => {
            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;

            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(NotationPatternPart::Cat(cat))
        },
        notation_pat_binding ::= [at, binding_kw, l_paren, cat_name_node, r_paren] => {
            debug_assert!(at.is_lit(*strings::AT));
            debug_assert!(binding_kw.is_kw(*strings::BINDING));
            debug_assert!(l_paren.is_lit(*strings::LEFT_PAREN));
            debug_assert!(r_paren.is_lit(*strings::RIGHT_PAREN));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            Ok(NotationPatternPart::Binding(cat))
        }
    }
}

fn elaborate_definition<'ctx>(
    definition: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Scope<'ctx>> {
    // definition_command ::= (definition) kw"definition" notation_binding ":=" fragment kw"end"

    match_rule! { (ctx, definition) =>
        definition ::= [definition_kw, notation_binding, assign, fragment_node, end_kw] => {
            debug_assert!(definition_kw.is_kw(*strings::DEFINITION));
            debug_assert!(assign.is_lit(*strings::ASSIGN));
            debug_assert!(end_kw.is_kw(*strings::END));

            let binding_possibilities = elaborate_notation_binding(notation_binding.as_node().unwrap(), ctx)?;
            let possible_frag_cats = elaborate_any_fragment(fragment_node.as_node().unwrap());

            let mut solution = None;
            for (cat, frag) in possible_frag_cats {
                let matching_bindings = binding_possibilities.get(cat);
                if matching_bindings.is_empty() { continue; }
                if matching_bindings.len() > 1 { todo!("Error: ambiguous notation binding.")}

                let (binding, holes) = &matching_bindings[0];

                let mut scope = scope.clone();
                for (i, (hole_cat, hole_name)) in holes.iter().enumerate() {
                    let binding = NotationBinding::new(ctx.single_name_notations[hole_cat], vec![NotationInstantiationPart::Name(*hole_name)]);
                    let binding = ctx.arenas.notation_bindings.intern(binding);
                    let entry = ScopeEntry::new_hole(*hole_cat, i);
                    scope = scope.child_with(binding, entry);
                }

                let frag = UnresolvedFrag(frag);
                let Ok(parse) = parse_fragment(frag, &scope, ctx)? else {
                    // TODO: do something with the errors here.
                    continue;
                };

                solution = Some((*binding, parse));
            }

            match solution {
                Some((binding, frag)) => {
                    let entry = ScopeEntry::new(frag);
                    Ok(scope.child_with(binding, entry))
                },
                None => todo!("Error: no matching notation bindings."),
            }
        }
    }
}

fn elaborate_any_fragment<'ctx>(
    any_frag: ParseTreeId<'ctx>,
) -> impl Iterator<Item = (FormalSyntaxCatId<'ctx>, ParseTreeId<'ctx>)> {
    any_frag.0.possibilities().iter().map(|possibility| {
        let frag = possibility.children()[0];
        let frag = frag.as_node().unwrap();
        let SyntaxCategorySource::FormalLang(formal_cat) = frag.cat().source() else {
            unreachable!();
        };

        (formal_cat, frag)
    })
}

fn elaborate_notation_binding<'ctx>(
    notation_binding: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<CatMap<'ctx, (NotationBindingId<'ctx>, Vec<(FormalSyntaxCatId<'ctx>, Ustr)>)>> {
    fn children_to_binding<'ctx>(
        children: &ParseTreeChildren<'ctx>,
        pattern: NotationPatternId<'ctx>,
        ctx: &mut Ctx<'ctx>,
    ) -> (NotationBindingId<'ctx>, Vec<(FormalSyntaxCatId<'ctx>, Ustr)>) {
        let mut instantiations = Vec::new();
        let mut hole_names = Vec::new();
        for (child, part) in children.children().iter().zip(pattern.parts()) {
            match part {
                NotationPatternPart::Name => {
                    let name = elaborate_name(child.as_node().unwrap(), ctx).unwrap();
                    instantiations.push(NotationInstantiationPart::Name(name));
                }
                NotationPatternPart::Cat(cat) => {
                    let name = elaborate_name(child.as_node().unwrap(), ctx).unwrap();
                    hole_names.push((*cat, name));
                }
                _ => {}
            }
        }
        let binding = NotationBinding::new(pattern, instantiations);
        let binding = ctx.arenas.notation_bindings.intern(binding);
        (binding, hole_names)
    }

    let mut solution = CatMap::new();

    for possibility in notation_binding.possibilities() {
        let rule = possibility.rule();
        let &ParseRuleSource::Notation(notation) = rule.source() else {
            unreachable!();
        };

        let (binding, hole_names) = children_to_binding(possibility, notation, ctx);
        solution.insert(notation.cat(), (binding, hole_names));
    }

    Ok(solution)
}

fn elaborate_notation_binding_with_cat<'ctx>(
    notation_binding: ParseTreeId<'ctx>,
    cat: FormalSyntaxCatId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(NotationBindingId<'ctx>, Vec<(FormalSyntaxCatId<'ctx>, Ustr)>)> {
    let by_cat = elaborate_notation_binding(notation_binding, ctx)?;
    let solutions = by_cat.get(cat);

    if solutions.is_empty() {
        todo!("Error: No solutions to binding");
    }

    if solutions.len() > 1 {
        todo!("Error: ambiguous solution to binding");
    }

    Ok(solutions[0].clone())
}

fn templates_to_scope<'ctx>(
    templates: &[Template<'ctx>],
    parent_scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> Scope<'ctx> {
    let mut my_scope = parent_scope.clone();

    for (i, template) in templates.iter().enumerate() {
        let args = template
            .binding()
            .pattern()
            .parts()
            .iter()
            .filter_map(|part| match part {
                NotationPatternPart::Cat(cat) => Some(*cat),
                _ => None,
            })
            .enumerate()
            .map(|(i, cat)| {
                let frag = Fragment::new(cat, FragHead::Hole(i), Vec::new());
                ctx.arenas.fragments.intern(frag)
            })
            .collect();
        let frag = Fragment::new(template.cat(), FragHead::TemplateRef(i), args);
        let frag = ctx.arenas.fragments.intern(frag);
        let entry = ScopeEntry::new(frag);
        my_scope = my_scope.child_with(template.binding(), entry)
    }

    my_scope
}

fn parse_hypotheses_and_conclusion<'ctx>(
    un_hypotheses: Vec<UnresolvedFact<'ctx>>,
    un_conclusion: UnresolvedFrag<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<(Vec<Fact<'ctx>>, FragmentId<'ctx>)> {
    let mut hypotheses = Vec::new();
    for un_hypothesis in un_hypotheses {
        let assumption = match un_hypothesis.assumption {
            Some(assumption) => match parse_fragment(assumption, scope, ctx)? {
                Ok(assumption) => Some(assumption),
                Err(err) => todo!("Error: failed to parse conclusion {err:?}"),
            },
            None => None,
        };
        let conclusion = match parse_fragment(un_hypothesis.conclusion, scope, ctx)? {
            Ok(conclusion) => conclusion,
            Err(err) => todo!("Error: failed to parse conclusion {err:?}"),
        };
        hypotheses.push(Fact::new(assumption, conclusion));
    }

    let conclusion = match parse_fragment(un_conclusion, scope, ctx)? {
        Ok(conclusion) => conclusion,
        Err(err) => todo!("Error: failed to parse conclusion {err:?}"),
    };

    Ok((hypotheses, conclusion))
}

fn elaborate_axiom<'ctx>(
    axiom: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<TheoremId<'ctx>> {
    // axiom_command ::= (axiom) kw"axiom" name templates ":" hypotheses "|-" sentence kw"end"

    match_rule! { (ctx, axiom) =>
        axiom ::= [axiom_kw, name_node, templates, colon, hypotheses, turnstile, conclusion, end_kw] => {
            debug_assert!(axiom_kw.is_kw(*strings::AXIOM));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(turnstile.is_lit(*strings::TURNSTILE));
            debug_assert!(end_kw.is_kw(*strings::END));

            let name = elaborate_name(name_node.as_node().unwrap(), ctx)?;
            let templates = elaborate_templates(templates.as_node().unwrap(), ctx)?;

            let my_scope = templates_to_scope(&templates, scope, ctx);

            let hypotheses = elaborate_hypotheses(hypotheses.as_node().unwrap(), ctx)?;
            let conclusion = UnresolvedFrag(conclusion.as_node().unwrap());

            let (hypotheses, conclusion) = parse_hypotheses_and_conclusion(hypotheses, conclusion, &my_scope, ctx)?;

            let scope_id = ctx.scopes.alloc(my_scope);

            let theorem_stmt = TheoremStatement::new(name, templates, hypotheses, conclusion, scope_id, UnresolvedProof::Axiom);
            let theorem_stmt = ctx.arenas.theorem_stmts.alloc(name, theorem_stmt);

            Ok(theorem_stmt)
        }
    }
}

fn elaborate_theorem<'ctx>(
    theorem: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<TheoremId<'ctx>> {
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

            let my_scope = templates_to_scope(&templates, scope, ctx);

            let hypotheses = elaborate_hypotheses(hypotheses.as_node().unwrap(), ctx)?;
            let conclusion = UnresolvedFrag(conclusion.as_node().unwrap());

            let (hypotheses, conclusion) = parse_hypotheses_and_conclusion(hypotheses, conclusion, &my_scope, ctx)?;

            let scope_id = ctx.scopes.alloc(my_scope);

            let proof = UnresolvedProof::Theorem(tactic.as_node().unwrap());

            let theorem_stmt = TheoremStatement::new(name, templates, hypotheses, conclusion, scope_id, proof);
            let theorem_stmt = ctx.arenas.theorem_stmts.alloc(name, theorem_stmt);

            Ok(theorem_stmt)
        }
    }
}

fn elaborate_templates<'ctx>(
    mut templates: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Vec<Template<'ctx>>> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<Vec<Template<'ctx>>> {
    // template ::= (template) "[" template_bindings ":" name "]"

    match_rule! { (ctx, template) =>
        template ::= [l_brack, names, colon, cat_name_node, r_brack] => {
            debug_assert!(l_brack.is_lit(*strings::LEFT_BRACKET));
            debug_assert!(colon.is_lit(*strings::COLON));
            debug_assert!(r_brack.is_lit(*strings::RIGHT_BRACKET));

            let cat_name = elaborate_name(cat_name_node.as_node().unwrap(), ctx)?;
            let Some(cat) = ctx.arenas.formal_cats.get(cat_name) else {
                return ctx.diags.err_unknown_formal_syntax_cat(cat_name, cat_name_node.span());
            };

            let bindings = elaborate_template_bindings(names.as_node().unwrap(), cat, ctx)?;

            Ok(bindings)
        }
    }
}

fn elaborate_template_bindings<'ctx>(
    mut bindings: ParseTreeId<'ctx>,
    cat: FormalSyntaxCatId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Vec<Template<'ctx>>> {
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
                let (binding, holes) = elaborate_notation_binding_with_cat(binding, cat, ctx)?;
                let template = Template::new(cat, binding, holes);

                binding_list.push(template);
                bindings = rest.as_node().unwrap();
            }
        }
    }
}

fn elaborate_hypotheses<'ctx>(
    hypotheses: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Vec<UnresolvedFact<'ctx>>> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<UnresolvedFact<'ctx>> {
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
    ctx: &mut Ctx<'ctx>,
) -> WResult<UnresolvedFact<'ctx>> {
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

// pub fn elaborate_tactic<'ctx>(
//     tactic: ParseTreeId<'ctx>,
//     ctx: &mut Ctx<'ctx>,
// ) -> WResult<UnresolvedTactic<'ctx>> {
// tactic ::= (tactic_none)
//          | (tactic_have) kw"have" fact tactics ";" tactics
//          | (tactic_by)   kw"by" name template_instantiations
//          | (tactic_todo) kw"todo"

// match_rule! { (ctx, tactic) =>
//     tactic_none ::= [] => {
//         Ok(UnresolvedTactic::None)
//     },
//     tactic_have ::= [have_kw, fact, proof, semicolon, continuation] => {
//         debug_assert!(have_kw.is_kw(*strings::HAVE));
//         debug_assert!(semicolon.is_lit(*strings::SEMICOLON));

//         let fact = elaborate_fact(fact.as_node( ).unwrap(), ctx)?;
//         let tactic = UnresolvedHaveTactic { fact, proof: proof.as_node().unwrap(), continuation: continuation.as_node().unwrap() };
//         Ok(UnresolvedTactic::Have(tactic))
//     },
//     tactic_by ::= [by_kw, theorem_name_node, template_insts] => {
//         debug_assert!(by_kw.is_kw(*strings::BY));

//         let theorem_name = elaborate_name(theorem_name_node.as_node().unwrap(), ctx)?;
//         let templates = elaborate_template_instantiations(template_insts.as_node().unwrap(), ctx)?;

//         let tactic = UnresolvedByTactic { theorem_name, theorem_name_span: theorem_name_node.span(), templates };
//         Ok(UnresolvedTactic::By(tactic))
//     },
//     tactic_todo ::= [todo_kw] => {
//         debug_assert!(todo_kw.is_kw(*strings::TODO));

//         Ok(UnresolvedTactic::Todo)
//     }
// }
// }

fn elaborate_template_instantiations<'ctx>(
    mut insts: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Vec<ParseTreeId<'ctx>>> {
    // template_instantiations ::= (template_instantiations_none)
    //                           | (template_instantiations_many) template_instantiation template_instantiations

    let mut insts_list = Vec::new();

    loop {
        match_rule! { (ctx, insts) =>
            template_instantiations_none ::= [] => {
                return Ok(insts_list);
            },
            template_instantiations_many ::= [inst, rest] => {
                let inst = inst.as_node().unwrap();
                insts_list.push(elaborate_template_instantiation(inst, ctx)?);
                insts = rest.as_node().unwrap();
            }
        }
    }
}

fn elaborate_template_instantiation<'ctx>(
    inst: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<ParseTreeId<'ctx>> {
    // template_instantiation ::= (template_instantiation) "[" any_fragment "]"

    match_rule! { (ctx, inst) =>
        template_instantiation ::= [l_brack, frag, r_brack] => {
            debug_assert!(l_brack.is_lit(*strings::LEFT_BRACKET));
            debug_assert!(r_brack.is_lit(*strings::RIGHT_BRACKET));

            let frag: ParseTreeId<'ctx> = frag.as_node().unwrap();
            debug_assert!(frag.cat() == ctx.builtin_cats.any_fragment);

            Ok(frag)
        }
    }
}

pub fn elaborate_name<'ctx>(name: ParseTreeId<'ctx>, ctx: &mut Ctx<'ctx>) -> WResult<Ustr> {
    match_rule! { (ctx, name) =>
        name ::= [name_atom] => {
            let name = name_atom.as_name().unwrap();
            Ok(name)
        }
    }
}

pub fn elaborate_str_lit<'ctx>(str_lit: ParseTreeId<'ctx>, ctx: &mut Ctx<'ctx>) -> WResult<Ustr> {
    match_rule! { (ctx, str_lit) =>
        str ::= [str_atom] => {
            let str_lit = str_atom.as_str_lit().unwrap();
            Ok(str_lit)
        }
    }
}

fn expect_unambiguous<'ctx>(
    id: ParseTreeId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<&'ctx ParseTreeChildren<'ctx>> {
    match id.0.possibilities() {
        [] => unreachable!("No possibilities in parse tree."),
        [possibility] => Ok(possibility),
        _ => ctx.diags.err_ambiguous_parse(id.span()),
    }
}
