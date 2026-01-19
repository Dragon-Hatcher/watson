use crate::{
    context::Ctx,
    generate_arena_handle,
    semant::{
        formal_syntax::FormalSyntaxPatPart,
        fragment::{FragHead, Fragment, FragmentId},
        notation::{NotationBindingId, NotationPatternPart},
    },
};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresFrag<'ctx> {
    /// The formal fragment this is a presentation of.
    frag: FragmentId<'ctx>,
    /// The presentation of the fragment.
    pres: PresId<'ctx>,
    /// The presentation for the same fragment but with all user defined notation
    /// replaced with formal syntax.
    formal: PresId<'ctx>,
}

impl<'ctx> PresFrag<'ctx> {
    pub fn new(frag: FragmentId<'ctx>, pres: PresId<'ctx>, formal: PresId<'ctx>) -> Self {
        Self { frag, pres, formal }
    }

    pub fn frag(&self) -> FragmentId<'ctx> {
        self.frag
    }

    pub fn pres(&self) -> PresId<'ctx> {
        self.pres
    }

    pub fn formal(&self) -> Self {
        // Since the formal presentation already contains only formal nodes,
        // the formal presentation of that tree will be the same.
        Self {
            frag: self.frag,
            pres: self.formal,
            formal: self.formal,
        }
    }

    pub fn formal_pres(&self) -> PresId<'ctx> {
        self.formal
    }

    pub fn print(&self) -> String {
        self.pres().print()
    }
}

generate_arena_handle! {PresId<'ctx> => Pres<'ctx>}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pres<'ctx> {
    /// The node value in the presentation tree.
    head: PresHead<'ctx>,
    /// The children in the presentation tree.
    children: Vec<PresFrag<'ctx>>,
}

impl<'ctx> Pres<'ctx> {
    pub fn new(head: PresHead<'ctx>, children: Vec<PresFrag<'ctx>>) -> Self {
        Self { head, children }
    }

    pub fn head(&self) -> PresHead<'ctx> {
        self.head
    }

    pub fn children(&self) -> &[PresFrag<'ctx>] {
        &self.children
    }

    pub fn print(&self) -> String {
        match self.head() {
            PresHead::FormalFrag(FragHead::Hole(idx)) => {
                if !self.children().is_empty() {
                    format!(
                        "_{idx}({})",
                        self.children().iter().map(|c| c.print()).join(", ")
                    )
                } else {
                    format!("_{idx}")
                }
            }
            PresHead::FormalFrag(FragHead::VarHole(idx)) => {
                format!("\"{idx}")
            }
            PresHead::FormalFrag(FragHead::Var(idx)) => {
                format!("'{idx}")
            }
            PresHead::FormalFrag(FragHead::TemplateRef(idx)) => {
                if !self.children().is_empty() {
                    format!(
                        "${idx}({})",
                        self.children().iter().map(|c| c.print()).join(", ")
                    )
                } else {
                    format!("${idx}")
                }
            }
            PresHead::FormalFrag(FragHead::RuleApplication(rule_app)) => {
                let mut out = String::new();
                let mut children = self.children().iter();

                for part in rule_app.rule().pattern().parts() {
                    use FormalSyntaxPatPart as P;

                    match part {
                        P::Lit(lit) => out.push_str(lit),
                        P::Binding(_) => out.push('_'),
                        P::Cat(_) => out.push_str(&children.next().unwrap().print()),
                    }
                }

                out
            }
            PresHead::Notation(binding, _) => {
                let mut out = String::new();
                let mut children = self.children().iter();
                let mut name_instantiations = binding.name_instantiations().iter();

                for part in binding.pattern().parts() {
                    use NotationPatternPart as P;

                    match part {
                        P::Lit(lit) => out.push_str(lit),
                        P::Kw(kw) => out.push_str(kw),
                        P::Name => out.push_str(name_instantiations.next().unwrap()),
                        P::Cat(_) => out.push_str(&children.next().unwrap().print()),
                        P::Binding(_) => {
                            // TODO
                            out.push_str("?");
                        }
                    }
                }

                out
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresHead<'ctx> {
    /// The notation for the fragment is directly a formal syntax fragment.
    FormalFrag(FragHead<'ctx>),
    /// The notation for the fragment is a notation binding which is replaced
    /// by the given PresFrag when instantiated.
    Notation(NotationBindingId<'ctx>, PresFrag<'ctx>),
}

impl<'ctx> PresHead<'ctx> {
    fn bindings_added(&self) -> usize {
        match self {
            PresHead::FormalFrag(head) => head.bindings_added(),
            PresHead::Notation(_, _) => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PresInstTy {
    Normal,
    Formal,
}

fn shift_frag<'ctx>(frag: FragmentId<'ctx>, shift: usize, ctx: &Ctx<'ctx>) -> FragmentId<'ctx> {
    fn inner<'ctx>(
        frag: FragmentId<'ctx>,
        shift: usize,
        closed_count: usize,
        ctx: &Ctx<'ctx>,
        memo: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
    ) -> FragmentId<'ctx> {
        // If this condition holds all the remaining variables refer to binders
        // inside the original fragment and so shouldn't be shifted.
        if frag.unclosed_vars() <= closed_count {
            return frag;
        }

        if let Some(cached) = memo.get(&(frag, closed_count)) {
            return *cached;
        }

        let new_frag = match frag.head() {
            FragHead::Var(idx) => {
                // The condition above ensures that this variables should be
                // shifted. So we do it unconditionally here.
                let head = FragHead::Var(idx + shift);
                let frag = Fragment::new(frag.cat(), head, Vec::new());
                ctx.arenas.fragments.intern(frag)
            }
            _ => {
                // This head might add new binders to the children.
                let closed_count = closed_count + frag.head().bindings_added();

                let new_children = frag
                    .children()
                    .iter()
                    .map(|&c| inner(c, shift, closed_count, ctx, memo))
                    .collect();
                let frag = Fragment::new(frag.cat(), frag.head(), new_children);
                ctx.arenas.fragments.intern(frag)
            }
        };
        memo.insert((frag, closed_count), new_frag);
        new_frag
    }

    // We keep track of binders internally because variables that refer to
    // bindings inside the terms are closed and shouldn't be shifted.
    inner(frag, shift, 0, ctx, &mut FxHashMap::default())
}

fn instantiate_frag_var_holes<'ctx>(
    frag: FragmentId<'ctx>,
    binding_depth: usize,
    hole_arg_offset: usize,
    hole_count: usize,
    holes: &impl Fn(usize) -> FragmentId<'ctx>,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
) -> FragmentId<'ctx> {
    if !frag.has_var_hole() {
        return frag;
    }

    if let Some(cached) = frag_cache.get(&(frag, binding_depth)) {
        return *cached;
    }

    let new_frag = match frag.head() {
        FragHead::VarHole(idx) => {
            if hole_arg_offset <= idx && idx < hole_arg_offset + hole_count {
                let replacement = holes(idx - hole_arg_offset);
                shift_frag(replacement, binding_depth, ctx)
            } else {
                frag
            }
        }
        FragHead::RuleApplication(_) | FragHead::TemplateRef(_) | FragHead::Hole { .. } => {
            let binding_depth = binding_depth + frag.head().bindings_added();
            let new_children = frag
                .children()
                .iter()
                .map(|&child| {
                    instantiate_frag_var_holes(
                        child,
                        binding_depth,
                        hole_arg_offset,
                        hole_count,
                        holes,
                        ctx,
                        frag_cache,
                    )
                })
                .collect();
            let frag = Fragment::new(frag.cat(), frag.head(), new_children);
            ctx.arenas.fragments.intern(frag)
        }
        FragHead::Var(_) => {
            debug_assert!(frag.children().is_empty());
            frag
        }
    };
    frag_cache.insert((frag, binding_depth), new_frag);
    new_frag
}

fn instantiate_frag_holes<'ctx>(
    frag: FragmentId<'ctx>,
    binding_depth: usize,
    holes: &impl Fn(usize) -> FragmentId<'ctx>,
    hole_arg_offset: usize,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
) -> FragmentId<'ctx> {
    if !frag.has_hole() {
        return frag;
    }

    if let Some(cached) = frag_cache.get(&(frag, binding_depth)) {
        return *cached;
    }

    let mut new_children = |bd: usize| {
        frag.children()
            .iter()
            .map(|&child| {
                instantiate_frag_holes(child, bd, holes, hole_arg_offset, ctx, frag_cache)
            })
            .collect_vec()
    };

    let new_frag = match frag.head() {
        FragHead::Hole(idx) => {
            let replacement = holes(idx);
            let replacement = shift_frag(replacement, binding_depth, ctx);
            let new_children = new_children(binding_depth);
            instantiate_frag_var_holes(
                replacement,
                0,
                hole_arg_offset,
                new_children.len(),
                &|idx| new_children[idx],
                ctx,
                &mut FxHashMap::default(),
            )
        }
        FragHead::RuleApplication(_) | FragHead::TemplateRef(_) => {
            let binding_depth = binding_depth + frag.head().bindings_added();
            let new_children = new_children(binding_depth);
            let frag = Fragment::new(frag.cat(), frag.head(), new_children);
            ctx.arenas.fragments.intern(frag)
        }
        FragHead::VarHole(_) | FragHead::Var(_) => {
            debug_assert!(frag.children().is_empty());
            frag
        }
    };
    frag_cache.insert((frag, binding_depth), new_frag);
    new_frag
}

fn instantiate_pres_holes<'ctx>(
    pres: PresId<'ctx>,
    binding_depth: usize,
    ty: PresInstTy,
    holes: &impl Fn(usize) -> PresFrag<'ctx>,
    hole_arg_offset: usize,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
    pres_cache: &mut FxHashMap<(PresId<'ctx>, PresInstTy), PresId<'ctx>>,
) -> PresId<'ctx> {
    if let Some(cached) = pres_cache.get(&(pres, ty)) {
        return *cached;
    }

    let new_pres = match pres.head() {
        PresHead::FormalFrag(FragHead::Hole(idx)) => match ty {
            // TODO: children, shift
            PresInstTy::Normal => holes(idx).pres(),
            PresInstTy::Formal => holes(idx).formal_pres(),
        },
        // TODO: check for holes in replacement.
        _ => {
            let binding_depth = binding_depth + pres.head().bindings_added();
            let new_children = pres
                .children()
                .iter()
                .map(|&child| {
                    instantiate_holes_impl(
                        child,
                        binding_depth,
                        ty,
                        holes,
                        hole_arg_offset,
                        ctx,
                        frag_cache,
                        pres_cache,
                    )
                })
                .collect();
            let pres = Pres::new(pres.head(), new_children);
            ctx.arenas.presentations.intern(pres)
        }
    };
    pres_cache.insert((pres, ty), new_pres);
    new_pres
}

fn instantiate_holes_impl<'ctx>(
    pres_frag: PresFrag<'ctx>,
    binding_depth: usize,
    ty: PresInstTy,
    holes: &impl Fn(usize) -> PresFrag<'ctx>,
    hole_arg_offset: usize,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
    pres_cache: &mut FxHashMap<(PresId<'ctx>, PresInstTy), PresId<'ctx>>,
) -> PresFrag<'ctx> {
    let normal = instantiate_pres_holes(
        pres_frag.pres(),
        binding_depth,
        PresInstTy::Normal,
        holes,
        hole_arg_offset,
        ctx,
        frag_cache,
        pres_cache,
    );
    let formal = instantiate_pres_holes(
        pres_frag.formal_pres(),
        binding_depth,
        PresInstTy::Formal,
        holes,
        hole_arg_offset,
        ctx,
        frag_cache,
        pres_cache,
    );

    PresFrag::new(
        instantiate_frag_holes(
            pres_frag.frag(),
            binding_depth,
            &|idx| holes(idx).frag(),
            hole_arg_offset,
            ctx,
            frag_cache,
        ),
        match ty {
            PresInstTy::Normal => normal,
            PresInstTy::Formal => formal,
        },
        formal,
    )
}

pub fn instantiate_holes<'ctx>(
    frag: PresFrag<'ctx>,
    holes: &impl Fn(usize) -> PresFrag<'ctx>,
    hole_arg_offset: usize,
    ctx: &Ctx<'ctx>,
) -> PresFrag<'ctx> {
    instantiate_holes_impl(
        frag,
        0,
        PresInstTy::Normal,
        holes,
        hole_arg_offset,
        ctx,
        &mut FxHashMap::default(),
        &mut FxHashMap::default(),
    )
}

fn instantiate_frag_templates<'ctx>(
    frag: FragmentId<'ctx>,
    binding_depth: usize,
    templates: &dyn Fn(usize) -> PresFrag<'ctx>,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
) -> FragmentId<'ctx> {
    if !frag.has_template() {
        return frag;
    }

    if let Some(cached) = frag_cache.get(&(frag, binding_depth)) {
        return *cached;
    }

    let mut new_children = |bd: usize| {
        frag.children()
            .iter()
            .map(|&child| instantiate_frag_templates(child, bd, templates, ctx, frag_cache))
            .collect_vec()
    };

    let new_frag = match frag.head() {
        FragHead::TemplateRef(idx) => {
            let replacement = templates(idx).frag();
            let replacement = shift_frag(replacement, binding_depth, ctx);
            // The children take the same binding depth. instantiate_frag_holes
            // will shift further if needed.
            let new_children = new_children(binding_depth);
            instantiate_frag_holes(
                replacement,
                0,
                &|idx| new_children[idx],
                0, // these holes should have no bindings so no need for an offset.
                ctx,
                &mut FxHashMap::default(),
            )
        }
        FragHead::RuleApplication(_) | FragHead::Hole { .. } => {
            let binding_depth = binding_depth + frag.head().bindings_added();
            let new_children = new_children(binding_depth);
            let frag = Fragment::new(frag.cat(), frag.head(), new_children);
            ctx.arenas.fragments.intern(frag)
        }
        FragHead::VarHole(_) | FragHead::Var(_) => {
            debug_assert!(frag.children().is_empty());
            frag
        }
    };
    frag_cache.insert((frag, binding_depth), new_frag);
    new_frag
}

fn instantiate_pres_templates<'ctx>(
    pres: PresId<'ctx>,
    binding_depth: usize,
    ty: PresInstTy,
    templates: &dyn Fn(usize) -> PresFrag<'ctx>,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
    pres_cache: &mut FxHashMap<(PresId<'ctx>, PresInstTy), PresId<'ctx>>,
) -> PresId<'ctx> {
    if let Some(cached) = pres_cache.get(&(pres, ty)) {
        return *cached;
    }

    let mut new_children = |bd: usize| {
        pres.children()
            .iter()
            .map(|&child| {
                instantiate_templates_impl(child, bd, ty, templates, ctx, frag_cache, pres_cache)
            })
            .collect_vec()
    };

    let new_pres = match pres.head() {
        PresHead::FormalFrag(FragHead::TemplateRef(idx)) => {
            let replacement = match ty {
                // TODO: shift
                PresInstTy::Normal => templates(idx).pres(),
                PresInstTy::Formal => templates(idx).formal_pres(),
            };
            let new_children = new_children(binding_depth);
            instantiate_pres_holes(
                replacement,
                0,
                ty,
                &|idx| new_children[idx],
                0, // these holes should have no bindings so no need for an offset.
                ctx,
                &mut FxHashMap::default(),
                &mut FxHashMap::default(),
            )
        }
        PresHead::Notation(_, replacement) if replacement.frag().has_template() => {
            // If the replacement for this notation contains template params
            // then we need to expand the notation. The notation isn't accurate
            // any more.
            let instantiated_replacement = instantiate_pres_holes(
                replacement.pres(),
                0,
                ty,
                &|idx| pres.children()[idx],
                0, // these holes should have no bindings so no need for an offset.
                ctx,
                &mut FxHashMap::default(),
                &mut FxHashMap::default(),
            );

            instantiate_pres_templates(
                instantiated_replacement,
                binding_depth,
                ty,
                templates,
                ctx,
                frag_cache,
                pres_cache,
            )
        }
        _ => {
            let binding_depth = binding_depth + pres.head().bindings_added();
            let new_children = new_children(binding_depth);
            let pres = Pres::new(pres.head(), new_children);
            ctx.arenas.presentations.intern(pres)
        }
    };
    pres_cache.insert((pres, ty), new_pres);
    new_pres
}

fn instantiate_templates_impl<'ctx>(
    pres_frag: PresFrag<'ctx>,
    binding_depth: usize,
    ty: PresInstTy,
    templates: &dyn Fn(usize) -> PresFrag<'ctx>,
    ctx: &Ctx<'ctx>,
    frag_cache: &mut FxHashMap<(FragmentId<'ctx>, usize), FragmentId<'ctx>>,
    pres_cache: &mut FxHashMap<(PresId<'ctx>, PresInstTy), PresId<'ctx>>,
) -> PresFrag<'ctx> {
    let normal = instantiate_pres_templates(
        pres_frag.pres(),
        binding_depth,
        PresInstTy::Normal,
        templates,
        ctx,
        frag_cache,
        pres_cache,
    );
    let formal = instantiate_pres_templates(
        pres_frag.formal_pres(),
        binding_depth,
        PresInstTy::Formal,
        templates,
        ctx,
        frag_cache,
        pres_cache,
    );

    PresFrag::new(
        instantiate_frag_templates(pres_frag.frag(), binding_depth, templates, ctx, frag_cache),
        match ty {
            PresInstTy::Normal => normal,
            PresInstTy::Formal => formal,
        },
        formal,
    )
}

pub fn instantiate_templates<'ctx>(
    frag: PresFrag<'ctx>,
    templates: &dyn Fn(usize) -> PresFrag<'ctx>,
    ctx: &Ctx<'ctx>,
) -> PresFrag<'ctx> {
    instantiate_templates_impl(
        frag,
        0,
        PresInstTy::Normal,
        templates,
        ctx,
        &mut FxHashMap::default(),
        &mut FxHashMap::default(),
    )
}

pub fn match_presentation<'ctx>(
    haystack: PresFrag<'ctx>,
    pattern: PresFrag<'ctx>,
) -> Option<FxHashMap<usize, PresFrag<'ctx>>> {
    fn inner<'ctx>(
        haystack: PresFrag<'ctx>,
        pattern: PresFrag<'ctx>,
        found_holes: &mut FxHashMap<usize, PresFrag<'ctx>>,
        already_checked: &mut FxHashSet<(PresFrag<'ctx>, PresFrag<'ctx>)>,
    ) -> bool {
        if already_checked.contains(&(haystack, pattern)) {
            return true;
        }

        if let PresHead::FormalFrag(FragHead::Hole(idx)) = pattern.pres().head() {
            if !pattern.frag().children().is_empty() {
                // Don't handle holes with children for now.
                return false;
            }

            // Insert the haystack as the solution for this hole or get the
            // previous solution.
            let previous = found_holes.entry(idx).or_insert(haystack);

            // Allow fragments that only match on formal and not on presentation.
            return previous.formal() == haystack.formal();
        } else if pattern.pres().head() != haystack.pres().head() {
            // The haystack and the pattern don't use the same notation at this node.
            return false;
        }

        already_checked.insert((haystack, pattern));

        // Now we recurse to our children.
        for (h_child, p_child) in haystack
            .pres()
            .children()
            .iter()
            .zip(pattern.pres().children())
        {
            if !inner(*h_child, *p_child, found_holes, already_checked) {
                return false;
            }
        }

        true
    }

    let mut holes = FxHashMap::default();
    let matches = inner(haystack, pattern, &mut holes, &mut FxHashSet::default());

    matches.then_some(holes)
}
