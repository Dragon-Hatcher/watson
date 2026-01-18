use crate::{
    context::{Ctx, arena::ScopeId},
    generate_arena_handle,
    semant::{
        formal_syntax::FormalSyntaxCatId,
        fragment::{Fact, FragHead, Fragment, hole_frag},
        notation::{_debug_binding, NotationBindingId},
        presentation::{Pres, PresFrag, PresHead},
        scope::{Scope, ScopeEntry},
    },
};
use itertools::Itertools;
use ustr::Ustr;

generate_arena_handle!(TheoremId<'ctx> => TheoremStatement<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremStatement<'ctx> {
    name: Ustr,
    templates: Vec<Template<'ctx>>,
    hypotheses: Vec<PresFact<'ctx>>,
    conclusion: PresFrag<'ctx>,
    scope: ScopeId,
}

impl<'ctx> TheoremStatement<'ctx> {
    pub fn new(
        name: Ustr,
        templates: Vec<Template<'ctx>>,
        hypotheses: Vec<PresFact<'ctx>>,
        conclusion: PresFrag<'ctx>,
        scope: ScopeId,
    ) -> Self {
        Self {
            name,
            templates,
            hypotheses,
            conclusion,
            scope,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn templates(&self) -> &[Template<'ctx>] {
        &self.templates
    }

    pub fn hypotheses(&self) -> &[PresFact<'ctx>] {
        &self.hypotheses
    }

    pub fn conclusion(&self) -> PresFrag<'ctx> {
        self.conclusion
    }

    pub fn scope(&self) -> ScopeId {
        self.scope
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    binding: NotationBindingId<'ctx>,
    holes: Vec<NotationBindingId<'ctx>>,
}

impl<'ctx> Template<'ctx> {
    pub fn new(
        cat: FormalSyntaxCatId<'ctx>,
        binding: NotationBindingId<'ctx>,
        holes: Vec<NotationBindingId<'ctx>>,
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

    pub fn holes(&self) -> &[NotationBindingId<'ctx>] {
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
        ctx: &Ctx<'ctx>,
    ) -> PresFrag<'ctx> {
        let holes = template.holes().iter().enumerate();
        let hole_pres_frags = holes
            .map(|(i, binding)| hole_frag(i, binding.pattern().cat(), Vec::new(), ctx))
            .collect_vec();
        let hole_frags = hole_pres_frags.iter().map(|f| f.frag()).collect();

        let frag = Fragment::new(template.cat(), FragHead::TemplateRef(idx), hole_frags);
        let frag = ctx.arenas.fragments.intern(frag);
        let pres = Pres::new(PresHead::FormalFrag(frag.head()), hole_pres_frags);
        let pres = ctx.arenas.presentations.intern(pres);

        // The presentation is already formal so we can pass the pres as the
        // formal pres.
        PresFrag::new(frag, pres, pres)
    }

    let mut my_scope = parent_scope.clone();

    for (i, template) in templates.iter().enumerate() {
        let frag = template_to_frag(template, i, ctx);
        let entry = ScopeEntry::new(frag);
        my_scope = my_scope.child_with(template.binding(), entry)
    }

    my_scope
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresFact<'ctx> {
    assumption: Option<PresFrag<'ctx>>,
    conclusion: PresFrag<'ctx>,
}

impl<'ctx> PresFact<'ctx> {
    pub fn new(assumption: Option<PresFrag<'ctx>>, conclusion: PresFrag<'ctx>) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }

    pub fn fact(&self) -> Fact<'ctx> {
        Fact::new(
            self.assumption().map(|a| a.frag()),
            self.conclusion().frag(),
        )
    }

    pub fn assumption(&self) -> Option<PresFrag<'ctx>> {
        self.assumption
    }

    pub fn conclusion(&self) -> PresFrag<'ctx> {
        self.conclusion
    }

    pub fn formal(&self) -> PresFact<'ctx> {
        Self::new(
            self.assumption().map(|a| a.formal()),
            self.conclusion().formal(),
        )
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
