use ustr::Ustr;

use crate::{
    generate_arena_handle,
    parse::parse_tree::ParseTreeId,
    semant::{formal_syntax::FormalSyntaxCatId, fragment::FragmentId},
};

generate_arena_handle!(TheoremId<'ctx> => TheoremStatement<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TheoremStatement<'ctx> {
    name: Ustr,
    templates: Vec<Template<'ctx>>,
    hypotheses: Vec<Fact<'ctx>>,
    conclusion: FragmentId<'ctx>,
    proof: UnresolvedProof<'ctx>,
}

impl<'ctx> TheoremStatement<'ctx> {
    pub fn new(
        name: Ustr,
        templates: Vec<Template<'ctx>>,
        hypotheses: Vec<Fact<'ctx>>,
        conclusion: FragmentId<'ctx>,
        proof: UnresolvedProof<'ctx>,
    ) -> Self {
        Self {
            name,
            templates,
            hypotheses,
            conclusion,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnresolvedProof<'ctx> {
    Axiom,
    Theorem(ParseTreeId<'ctx>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Template<'ctx> {
    name: Ustr,
    cat: FormalSyntaxCatId<'ctx>,
    params: Vec<FormalSyntaxCatId<'ctx>>,
}

impl<'ctx> Template<'ctx> {
    pub fn new(
        name: Ustr,
        cat: FormalSyntaxCatId<'ctx>,
        params: Vec<FormalSyntaxCatId<'ctx>>,
    ) -> Self {
        Self { name, cat, params }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn params(&self) -> &[FormalSyntaxCatId<'ctx>] {
        &self.params
    }
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
