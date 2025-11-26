use crate::{
    context::arena::ScopeId, generate_arena_handle, parse::parse_tree::ParseTreeId, semant::{
        formal_syntax::FormalSyntaxCatId, fragment::FragmentId, notation::NotationBindingId, presentation::{FactPresentation, PresentationTreeId}, scope::Scope
    }
};
use ustr::Ustr;

generate_arena_handle!(TheoremId<'ctx> => TheoremStatement<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremStatement<'ctx> {
    name: Ustr,
    templates: Vec<Template<'ctx>>,
    hypotheses: Vec<Fact<'ctx>>,
    conclusion: FragmentId<'ctx>,
    scope: ScopeId,
    proof: UnresolvedProof<'ctx>,
}

impl<'ctx> TheoremStatement<'ctx> {
    pub fn new(
        name: Ustr,
        templates: Vec<Template<'ctx>>,
        hypotheses: Vec<Fact<'ctx>>,
        conclusion: FragmentId<'ctx>,
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

    pub fn conclusion(&self) -> FragmentId<'ctx> {
        self.conclusion
    }

    pub fn proof(&self) -> &UnresolvedProof<'ctx> {
        &self.proof
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    binding: NotationBindingId<'ctx>,
    hole_names: Vec<Ustr>,
}

impl<'ctx> Template<'ctx> {
    pub fn new(cat: FormalSyntaxCatId<'ctx>, binding: NotationBindingId<'ctx>, hole_names: Vec<Ustr>) -> Self {
        Self {
            cat,
            binding,
            hole_names
        }
    }

    pub fn binding(&self) -> NotationBindingId<'ctx> {
        self.binding
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn hole_names(&self) -> &[Ustr] {
        &self.hole_names
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnresolvedProof<'ctx> {
    Axiom,
    Theorem(ParseTreeId<'ctx>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fact<'ctx> {
    assumption: Option<FragmentId<'ctx>>,
    conclusion: FragmentId<'ctx>,
}

impl<'ctx> Fact<'ctx> {
    pub fn new(assumption: Option<FragmentId<'ctx>>, conclusion: FragmentId<'ctx>) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }

    pub fn assumption(&self) -> Option<FragmentId<'ctx>> {
        self.assumption
    }

    pub fn conclusion(&self) -> FragmentId<'ctx> {
        self.conclusion
    }
}
