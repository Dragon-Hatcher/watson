use ustr::Ustr;

use crate::{
    context::arena::NamedArena,
    declare_intern_handle,
    parse::parse_tree::ParseTreeId,
    semant::{
        formal_syntax::FormalSyntaxCatId,
        fragment::{_debug_fragment, FragmentId},
    },
};

pub struct TheoremStatements<'ctx> {
    theorems: NamedArena<TheoremStatement<'ctx>, TheoremId<'ctx>>,
}

impl<'ctx> TheoremStatements<'ctx> {
    pub fn new() -> Self {
        Self {
            theorems: NamedArena::new(),
        }
    }

    pub fn add(&'ctx self, statement: TheoremStatement<'ctx>) -> TheoremId<'ctx> {
        assert!(self.theorems.get(statement.name).is_none());
        self.theorems.alloc(statement.name, statement)
    }

    pub fn get(&self, name: Ustr) -> Option<TheoremId> {
        self.theorems.get(name)
    }
}

declare_intern_handle!(TheoremId<'ctx> => TheoremStatement<'ctx>);

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

    pub fn cat(&self) -> FormalSyntaxCatId {
        self.cat
    }

    pub fn params(&self) -> &[FormalSyntaxCatId] {
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

pub fn _debug_theorem_statement(id: TheoremId, stmt: &TheoremStatement, ctx: &crate::Ctx) {
    println!("Theorem {}:", id.name());
    for template in stmt.templates() {
        if template.params().is_empty() {
            println!("  [{} : {}]", template.name(), template.cat().name(),);
        } else {
            println!(
                "  [{}({}) : {}]",
                template.name(),
                template
                    .params()
                    .iter()
                    .map(|cat| cat.name().as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                template.cat().name(),
            );
        }
    }
    for fact in stmt.hypotheses() {
        if let Some(assump) = fact.assumption() {
            println!(
                "  (assume {} |- {})",
                _debug_fragment(assump, ctx),
                _debug_fragment(fact.conclusion(), ctx)
            );
        } else {
            println!("  ({})", _debug_fragment(fact.conclusion(), ctx));
        }
    }
    println!("  |- {}", _debug_fragment(stmt.conclusion(), ctx));
}
