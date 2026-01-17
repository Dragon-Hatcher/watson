use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{elaborator::elaborate_name, parse_state::ParseRuleSource, parse_tree::ParseTreeId},
    semant::{
        formal_syntax::FormalSyntaxCatId,
        fragment::hole_frag,
        notation::{NotationBinding, NotationPatternPart},
        presentation::{Pres, PresFrag, PresHead, instantiate_holes},
        scope::{Scope, ScopeReplacement},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedFrag<'ctx>(pub ParseTreeId<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedAnyFrag<'ctx>(pub ParseTreeId<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedFact<'ctx> {
    pub assumption: Option<UnresolvedFrag<'ctx>>,
    pub conclusion: UnresolvedFrag<'ctx>,
}

pub fn parse_any_fragment<'ctx>(
    frag: UnresolvedAnyFrag<'ctx>,
    cat: FormalSyntaxCatId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Result<PresFrag<'ctx>, ParseResultErr>> {
    for possibility in frag.0.possibilities() {
        let ParseRuleSource::AnyFrag(p_cat) = possibility.rule().0.source() else {
            unreachable!();
        };

        if cat != *p_cat {
            continue;
        }

        let frag = possibility.children()[0].as_node().unwrap();
        let frag = UnresolvedFrag(frag);
        return parse_fragment(frag, scope, ctx);
    }

    Ok(Err(ParseResultErr::WrongCat))
}

pub fn parse_fragment<'ctx>(
    frag: UnresolvedFrag<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Result<PresFrag<'ctx>, ParseResultErr>> {
    parse_fragment_impl(frag.0, scope, ctx)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseResultErr {
    NoSolutions,
    MultipleSolutions,
    WrongCat,
}

fn parse_fragment_impl<'ctx>(
    frag: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, Result<PresFrag<'ctx>, ParseResultErr>> {
    let mut solution = Err(ParseResultErr::NoSolutions);

    'possibility: for possibility in frag.possibilities() {
        let rule = possibility.rule();
        let notation = rule.source().get_notation();

        // First let's create the binding that this syntax represented and
        // look it up in our scope. If it doesn't exist we can move on.
        let mut name_instantiations = Vec::new();
        for (child, part) in possibility.children().iter().zip(notation.parts()) {
            if let NotationPatternPart::Name = part {
                let name = elaborate_name(child.as_node().unwrap(), ctx)?;
                name_instantiations.push(name);
            }
        }
        let binding = NotationBinding::new(notation, name_instantiations);
        let binding = ctx.arenas.notation_bindings.intern(binding);

        let Some(replacement) = scope.lookup(binding) else {
            // If we didn't find anything then this notation isn't bound in this
            // scope so we should try the next possibility or error out.
            continue;
        };

        // Next we want to create a scope we can parse our children with that
        // contains the binders this notation introduced.
        // TODO: implement
        let new_scope = scope.clone();

        // Now we want to evaluate each of the child nodes in the context of
        // the new scope that we created.
        let mut children = Vec::new();
        let mut multiple_solutions = false;
        for (child, part) in possibility.children().iter().zip(notation.parts()) {
            if let NotationPatternPart::Cat(_child_cat) = part {
                let child_node = child.as_node().unwrap();
                let child_parse = parse_fragment_impl(child_node, &new_scope, ctx)?;
                match child_parse {
                    Ok(parse) => children.push(parse),
                    Err(ParseResultErr::NoSolutions) => {
                        continue 'possibility;
                    }
                    Err(ParseResultErr::MultipleSolutions) => multiple_solutions = true,
                    Err(ParseResultErr::WrongCat) => unreachable!(),
                }
            }
        }

        // We found multiple solutions in the child nodes.
        if multiple_solutions {
            return Ok(Err(ParseResultErr::MultipleSolutions));
        }

        // Now we perform the replacement using the children we have parsed.
        let instantiated = match replacement.replacement() {
            ScopeReplacement::Frag(replacement) => {
                let instantiated_replacement =
                    instantiate_holes(replacement, &|idx| children[idx], ctx);
                let my_pres = Pres::new(PresHead::Notation(binding, replacement), children);
                let my_pres = ctx.arenas.presentations.intern(my_pres);
                PresFrag::new(
                    instantiated_replacement.frag(),
                    my_pres,
                    instantiated_replacement.formal_pres(),
                )
            }
            ScopeReplacement::Hole(cat, idx) => hole_frag(idx, cat, ctx),
        };

        if let Ok(_alternate) = solution {
            return Ok(Err(ParseResultErr::MultipleSolutions));
        }

        solution = Ok(instantiated);
    }

    Ok(solution)
}
