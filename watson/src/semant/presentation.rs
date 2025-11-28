use rustc_hash::FxHashMap;

use crate::{
    context::Ctx,
    generate_arena_handle,
    semant::{
        fragment::FragmentId,
        notation::{NotationBindingId, NotationInstantiationPart, NotationPatternPart},
    },
};

generate_arena_handle! { PresId<'ctx> => Pres<'ctx> }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pres<'ctx> {
    head: PresHead<'ctx>,
    children: Vec<PresId<'ctx>>,
    has_hole: bool,
}

impl<'ctx> Pres<'ctx> {
    pub fn new(head: PresHead<'ctx>, children: Vec<PresId<'ctx>>) -> Self {
        let has_hole = matches!(head, PresHead::Hole(_)) || children.iter().any(|c| c.has_hole());
        Self {
            head,
            children,
            has_hole,
        }
    }

    pub fn head(&self) -> PresHead<'ctx> {
        self.head
    }

    pub fn children(&self) -> &[PresId<'ctx>] {
        &self.children
    }

    pub fn has_hole(&self) -> bool {
        self.has_hole
    }

    pub fn print(&self) -> String {
        match self.head() {
            PresHead::Notation(binding) => {
                let mut out = String::new();
                // if binding.pattern().parts().len() > 1 {
                //     out.push('(');
                // }

                let mut instantiations = binding.instantiations().iter();
                let mut children = self.children().iter();
                for (i, part) in binding.pattern().parts().iter().enumerate() {
                    if i != 0 {
                        out.push(' ');
                    }

                    match part {
                        NotationPatternPart::Lit(lit) => {
                            out.push_str(lit);
                        }
                        NotationPatternPart::Kw(kw) => {
                            out.push_str(kw);
                        }
                        NotationPatternPart::Name => match instantiations.next().unwrap() {
                            NotationInstantiationPart::Name(name) => {
                                out.push_str(name);
                            }
                        },
                        NotationPatternPart::Cat(_cat) => {
                            out.push_str(&children.next().unwrap().print())
                        }
                        NotationPatternPart::Binding(_binding) => todo!(),
                    }
                }

                // if binding.pattern().parts().len() > 1 {
                //     out.push(')');
                // }
                out
            }
            PresHead::Hole(idx) => format!("_{idx}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresHead<'ctx> {
    Notation(NotationBindingId<'ctx>),
    Hole(usize),
}

generate_arena_handle! { PresTreeId<'ctx> => PresTree<'ctx> }

/// Provides the presentation for each node of a formal syntax tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresTree<'ctx> {
    pres: PresId<'ctx>,
    expanded: Option<PresTreeId<'ctx>>,
    children: Vec<PresTreeId<'ctx>>,
    has_hole: bool,
}

impl<'ctx> PresTree<'ctx> {
    pub fn new_expanded(
        pres: PresId<'ctx>,
        expanded: PresTreeId<'ctx>,
        children: Vec<PresTreeId<'ctx>>,
    ) -> Self {
        let has_hole = pres.has_hole() || children.iter().any(|c| c.has_hole);
        Self {
            pres,
            expanded: Some(expanded),
            children,
            has_hole,
        }
    }

    pub fn new(pres: PresId<'ctx>, children: Vec<PresTreeId<'ctx>>) -> Self {
        let has_hole = pres.has_hole() || children.iter().any(|c| c.has_hole);
        Self {
            pres,
            expanded: None,
            children,
            has_hole,
        }
    }

    pub fn pres(&self) -> PresId<'ctx> {
        self.pres
    }

    pub fn expanded(&self) -> Option<PresTreeId<'ctx>> {
        self.expanded
    }

    pub fn children(&self) -> &[PresTreeId<'ctx>] {
        &self.children
    }

    pub fn has_hole(&self) -> bool {
        self.has_hole
    }
}

pub fn abstract_pres_tree_root<'ctx>(
    tree: PresTreeId<'ctx>,
    new_pres: PresId<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> PresTreeId<'ctx> {
    let new_tree = PresTree::new_expanded(new_pres, tree, tree.children.clone());
    ctx.arenas.presentation_trees.intern(new_tree)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresFrag<'ctx>(pub FragmentId<'ctx>, pub PresTreeId<'ctx>);

impl<'ctx> PresFrag<'ctx> {
    pub fn frag(&self) -> FragmentId<'ctx> {
        self.0
    }

    pub fn pres(&self) -> PresTreeId<'ctx> {
        self.1
    }

    pub fn print(&self) -> String {
        self.pres().pres().print()
    }
}

pub fn instantiate_pres_tree<'ctx>(
    tree: PresTreeId<'ctx>,
    children: &[PresFrag<'ctx>],
    ctx: &mut Ctx<'ctx>,
) -> PresTreeId<'ctx> {
    fn instantiate_in_pres<'ctx>(
        pres: PresId<'ctx>,
        children: &[PresFrag<'ctx>],
        cache: &mut FxHashMap<PresId<'ctx>, PresId<'ctx>>,
        ctx: &mut Ctx<'ctx>,
    ) -> PresId<'ctx> {
        if !pres.has_hole() {
            return pres;
        }

        if let Some(cached) = cache.get(&pres) {
            return *cached;
        }

        let solution = match pres.head {
            PresHead::Notation(binding) => {
                let new_children = pres
                    .children()
                    .iter()
                    .map(|&child| instantiate_in_pres(child, children, cache, ctx))
                    .collect();
                let new_pres = Pres::new(PresHead::Notation(binding), new_children);
                ctx.arenas.presentations.intern(new_pres)
            }
            PresHead::Hole(idx) => children[idx].pres().pres(),
        };
        cache.insert(pres, solution);

        solution
    }

    fn inner<'ctx>(
        tree: PresTreeId<'ctx>,
        children: &[PresFrag<'ctx>],
        cache: &mut FxHashMap<PresId<'ctx>, PresId<'ctx>>,
        ctx: &mut Ctx<'ctx>,
    ) -> PresTreeId<'ctx> {
        if !tree.has_hole() {
            return tree;
        }

        match tree.pres().head() {
            PresHead::Notation(_) => {
                let new_pres = instantiate_in_pres(tree.pres(), children, cache, ctx);
                let new_children = tree
                    .children()
                    .iter()
                    .map(|&c| inner(c, children, cache, ctx))
                    .collect();
                let new_tree = PresTree::new(new_pres, new_children);
                ctx.arenas.presentation_trees.intern(new_tree)
            }
            PresHead::Hole(idx) => children[idx].pres(),
        }
    }

    inner(tree, children, &mut FxHashMap::default(), ctx)
}
