use crate::{
    context::{Ctx, arena::ScopeId},
    generate_arena_handle,
    parse::parse_tree::ParseTreeId,
    semant::{
        formal_syntax::FormalSyntaxCatId,
        fragment::{FragHead, Fragment, FragmentId},
        notation::{_debug_binding, NotationBindingId},
        presentation::{Pres, PresFrag, PresHead, PresId, PresTree, PresTreeId},
        scope::{Scope, ScopeEntry},
    },
};
use ustr::Ustr;

generate_arena_handle!(TheoremId<'ctx> => TheoremStatement<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremStatement<'ctx> {
    name: Ustr,
    templates: Vec<Template<'ctx>>,
    hypotheses: Vec<Fact<'ctx>>,
    conclusion: PresFrag<'ctx>,
    scope: ScopeId,
    proof: UnresolvedProof<'ctx>,
}

impl<'ctx> TheoremStatement<'ctx> {
    pub fn new(
        name: Ustr,
        templates: Vec<Template<'ctx>>,
        hypotheses: Vec<Fact<'ctx>>,
        conclusion: PresFrag<'ctx>,
        scope: ScopeId,
        proof: UnresolvedProof<'ctx>,
    ) -> Self {
        Self {
            name,
            templates,
            hypotheses,
            conclusion,
            scope,
            proof,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn templates(&self) -> &[Template<'ctx>] {
        &self.templates
    }

    pub fn hypotheses(&self) -> &[Fact<'ctx>] {
        &self.hypotheses
    }

    pub fn conclusion(&self) -> PresFrag<'ctx> {
        self.conclusion
    }

    pub fn _proof(&self) -> &UnresolvedProof<'ctx> {
        &self.proof
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    binding: NotationBindingId<'ctx>,
    holes: Vec<(FormalSyntaxCatId<'ctx>, Ustr)>,
}

impl<'ctx> Template<'ctx> {
    pub fn new(
        cat: FormalSyntaxCatId<'ctx>,
        binding: NotationBindingId<'ctx>,
        holes: Vec<(FormalSyntaxCatId<'ctx>, Ustr)>,
    ) -> Self {
        Self {
            cat,
            binding,
            holes,
        }
    }

    pub fn binding(&self) -> NotationBindingId<'ctx> {
        self.binding
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn holes(&self) -> &[(FormalSyntaxCatId<'ctx>, Ustr)] {
        &self.holes
    }
}

pub fn add_templates_to_scope<'ctx>(
    templates: &[Template<'ctx>],
    parent_scope: &Scope<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> Scope<'ctx> {
    fn template_to_frag<'ctx>(
        template: &Template<'ctx>,
        idx: usize,
        ctx: &mut Ctx<'ctx>,
    ) -> FragmentId<'ctx> {
        let args = template
            .holes()
            .iter()
            .enumerate()
            .map(|(i, (cat, _name))| {
                let frag = Fragment::new(*cat, FragHead::Hole(i), Vec::new());
                ctx.arenas.fragments.intern(frag)
            })
            .collect();
        let frag = Fragment::new(template.cat(), FragHead::TemplateRef(idx), args);
        ctx.arenas.fragments.intern(frag)
    }

    fn template_to_pres<'ctx>(template: &Template<'ctx>, ctx: &mut Ctx<'ctx>) -> PresTreeId<'ctx> {
        let (children, trees): (Vec<PresId>, Vec<PresTreeId>) = template
            .holes()
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let pres = Pres::new(PresHead::Hole(i), Vec::new());
                let pres = ctx.arenas.presentations.intern(pres);
                let tree = PresTree::new(pres, Vec::new());
                let tree = ctx.arenas.presentation_trees.intern(tree);
                (pres, tree)
            })
            .unzip();

        let parent_pres = Pres::new(PresHead::Notation(template.binding), children);
        let parent_pres = ctx.arenas.presentations.intern(parent_pres);
        let parent_tree = PresTree::new(parent_pres, trees);
        ctx.arenas.presentation_trees.intern(parent_tree)
    }

    let mut my_scope = parent_scope.clone();

    for (i, template) in templates.iter().enumerate() {
        let frag = template_to_frag(template, i, ctx);
        let pres = template_to_pres(template, ctx);
        let entry = ScopeEntry::new(PresFrag(frag, pres));
        my_scope = my_scope.child_with(template.binding(), entry)
    }

    my_scope
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnresolvedProof<'ctx> {
    Axiom,
    Theorem(ParseTreeId<'ctx>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fact<'ctx> {
    assumption: Option<PresFrag<'ctx>>,
    conclusion: PresFrag<'ctx>,
}

impl<'ctx> Fact<'ctx> {
    pub fn new(assumption: Option<PresFrag<'ctx>>, conclusion: PresFrag<'ctx>) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }

    pub fn assumption(&self) -> Option<PresFrag<'ctx>> {
        self.assumption
    }

    pub fn conclusion(&self) -> PresFrag<'ctx> {
        self.conclusion
    }

    pub fn print(&self) -> String {
        match self.assumption() {
            Some(assumption) => format!("{} |- {}", assumption.print(), self.conclusion().print()),
            None => self.conclusion().print(),
        }
    }
}

pub fn _debug_theorem<'ctx>(theorem: TheoremId<'ctx>) -> String {
    let mut out = String::new();
    out.push_str(&format!("Theorem: {}\n", theorem.name()));
    for template in theorem.templates() {
        out.push_str(&format!(
            "  [{} : {}]\n",
            _debug_binding(template.binding()),
            template.cat().name(),
        ));
    }
    for hypothesis in theorem.hypotheses() {
        out.push_str(&format!("  ({})\n", hypothesis.print()));
    }
    out.push_str(&format!("  |- {}\n", theorem.conclusion().print()));

    out
}
