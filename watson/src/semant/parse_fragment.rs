use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{elaborator::elaborate_name, parse_tree::ParseTreeId},
    semant::{
        fragment::{FragHead, Fragment, FragmentId, hole_frag},
        notation::{NotationBinding, NotationPatternPart},
        presentation::{Pres, PresFrag, PresHead, PresId},
        scope::{Scope, ScopeEntry, ScopeReplacement},
    },
};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedFrag<'ctx>(pub ParseTreeId<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedAnyFrag<'ctx>(pub ParseTreeId<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedFact<'ctx> {
    pub assumption: Option<UnresolvedFrag<'ctx>>,
    pub conclusion: UnresolvedFrag<'ctx>,
}

pub fn parse_fragment<'ctx>(
    frag: UnresolvedFrag<'ctx>,
    scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Result<PresFrag<'ctx>, ParseResultErr>> {
    parse_fragment_impl(frag.0, scope, 0, ctx)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseResultErr {
    NoSolutions,
    MultipleSolutions,
}

fn parse_fragment_impl<'ctx>(
    frag: ParseTreeId<'ctx>,
    scope: &Scope<'ctx>,
    binding_depth: usize,
    ctx: &mut Ctx<'ctx>,
) -> WResult<Result<PresFrag<'ctx>, ParseResultErr>> {
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
        let mut new_scope = scope.clone();
        let mut new_depth = binding_depth;

        // Check each of the child nodes in the _parse tree_. The notation
        // definition tells us which of them are binding nodes.
        for (child, part) in possibility.children().iter().zip(notation.parts()) {
            if let NotationPatternPart::Binding(cat) = part {
                // Extract the name of this binding.
                let name = elaborate_name(child.as_node().unwrap(), ctx)?;

                // Now we create the fragment this node will get replaced with.
                // We set the deBruijn index to always be zero. This will be
                // adjusted later by the binding depth.
                let head = FragHead::Variable(*cat, 0);
                let frag = Fragment::new(*cat, head, Vec::new());
                let frag = ctx.arenas.fragments.intern(frag);

                // The entry contains the fragment we just created but also the
                // binding depth which tells child nodes who read this binding
                // how many intermediate bindings there are so that they can fix
                // the node for their context.
                let entry = ScopeEntry::new_with_depth(todo!(), new_depth);
                new_depth += 1;

                // Finally we need the notation for a single name.
                let name_pattern = ctx.single_name_notations[cat];
                let bind_binding = NotationBinding::new(name_pattern, vec![name]);
                let bind_binding = ctx.arenas.notation_bindings.intern(bind_binding);

                // And now we can update the scope.
                new_scope = new_scope.child_with(bind_binding, entry);
            }
        }

        // Now we want to evaluate each of the child nodes in the context of
        // the new scope that we created.
        let mut children = Vec::new();
        let mut multiple_solutions = false;
        for (child, part) in possibility.children().iter().zip(notation.parts()) {
            if let NotationPatternPart::Cat(_child_cat) = part {
                let child_node = child.as_node().unwrap();
                let child_parse = parse_fragment_impl(child_node, &new_scope, new_depth, ctx)?;
                match child_parse {
                    Ok(parse) => children.push(parse),
                    Err(ParseResultErr::NoSolutions) => {
                        continue 'possibility;
                    }
                    Err(ParseResultErr::MultipleSolutions) => multiple_solutions = true,
                }
            }
        }

        // We found multiple solutions in the child nodes.
        if multiple_solutions {
            return Ok(Err(ParseResultErr::MultipleSolutions));
        }

        // Now we perform the replacement using the children we have parsed.
        let _intermediates = binding_depth - replacement.binding_depth();
        let instantiated = match replacement.replacement() {
            ScopeReplacement::Frag(replacement) => {
                let instantiated_replacement = instantiate_replacement(replacement, &children, ctx);
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

fn instantiate_replacement<'ctx>(
    frag: PresFrag<'ctx>,
    children: &[PresFrag<'ctx>],
    ctx: &Ctx<'ctx>,
) -> PresFrag<'ctx> {
    fn instantiate_frag<'ctx>(
        frag: FragmentId<'ctx>,
        children: &[PresFrag<'ctx>],
        ctx: &Ctx<'ctx>,
        frag_cache: &mut FxHashMap<FragmentId<'ctx>, FragmentId<'ctx>>,
    ) -> FragmentId<'ctx> {
        if !frag.has_hole() {
            return frag;
        }

        if let Some(cached) = frag_cache.get(&frag) {
            return *cached;
        }

        let new_frag = match frag.head() {
            FragHead::RuleApplication(_) | FragHead::TemplateRef(_) => {
                let new_children = frag
                    .children()
                    .iter()
                    .map(|&child| instantiate_frag(child, children, ctx, frag_cache))
                    .collect();
                let frag = Fragment::new(frag.cat(), frag.head(), new_children);
                ctx.arenas.fragments.intern(frag)
            }
            FragHead::Variable(_var, _) => todo!(),
            FragHead::Hole(idx) => children[idx].frag(),
        };
        frag_cache.insert(frag, new_frag);
        new_frag
    }

    fn instantiate_pres<'ctx>(
        pres: PresId<'ctx>,
        children: &[PresFrag<'ctx>],
        ctx: &Ctx<'ctx>,
        frag_cache: &mut FxHashMap<FragmentId<'ctx>, FragmentId<'ctx>>,
        pres_cache: &mut FxHashMap<PresId<'ctx>, PresId<'ctx>>,
    ) -> PresId<'ctx> {
        if let Some(cached) = pres_cache.get(&pres) {
            return *cached;
        }

        let new_pres = match pres.head() {
            PresHead::FormalFrag(FragHead::Hole(idx)) => children[idx].pres(),
            PresHead::FormalFrag(FragHead::Variable(_, _)) => todo!(),
            _ => {
                let new_children = pres
                    .children()
                    .iter()
                    .map(|&child| inner(child, children, ctx, frag_cache, pres_cache))
                    .collect();
                let pres = Pres::new(pres.head(), new_children);
                ctx.arenas.presentations.intern(pres)
            }
        };
        pres_cache.insert(pres, new_pres);
        new_pres
    }

    fn inner<'ctx>(
        pres_frag: PresFrag<'ctx>,
        children: &[PresFrag<'ctx>],
        ctx: &Ctx<'ctx>,
        frag_cache: &mut FxHashMap<FragmentId<'ctx>, FragmentId<'ctx>>,
        pres_cache: &mut FxHashMap<PresId<'ctx>, PresId<'ctx>>,
    ) -> PresFrag<'ctx> {
        PresFrag::new(
            instantiate_frag(pres_frag.frag(), children, ctx, frag_cache),
            instantiate_pres(pres_frag.pres(), children, ctx, frag_cache, pres_cache),
            instantiate_pres(
                pres_frag.formal_pres(),
                children,
                ctx,
                frag_cache,
                pres_cache,
            ),
        )
    }

    inner(
        frag,
        children,
        ctx,
        &mut FxHashMap::default(),
        &mut FxHashMap::default(),
    )
}
