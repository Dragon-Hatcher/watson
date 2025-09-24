use std::ops::Index;

use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::{
    parse::parse_tree::ParseTreeId,
    semant::{
        formal_syntax::FormalSyntaxCatId,
        fragment::{_debug_fragment, FragmentId},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TheoremId(Ustr);

impl TheoremId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }

    pub fn name(&self) -> Ustr {
        self.0
    }
}

pub struct TheoremStatements {
    theorems: FxHashMap<TheoremId, TheoremStatement>,
}

impl TheoremStatements {
    pub fn new() -> Self {
        Self {
            theorems: FxHashMap::default(),
        }
    }

    pub fn get(&self, id: TheoremId) -> Option<&TheoremStatement> {
        self.theorems.get(&id)
    }

    pub fn add(&mut self, id: TheoremId, statement: TheoremStatement) {
        self.theorems.insert(id, statement);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TheoremId, &TheoremStatement)> {
        self.theorems.iter()
    }
}

impl Index<TheoremId> for TheoremStatements {
    type Output = TheoremStatement;

    fn index(&self, index: TheoremId) -> &Self::Output {
        &self.theorems[&index]
    }
}

#[derive(Debug, Clone)]
pub struct TheoremStatement {
    templates: Vec<Template>,
    hypotheses: Vec<Fact>,
    conclusion: FragmentId,
    proof: UnresolvedProof,
}

impl TheoremStatement {
    pub fn new(
        templates: Vec<Template>,
        hypotheses: Vec<Fact>,
        conclusion: FragmentId,
        proof: UnresolvedProof,
    ) -> Self {
        Self {
            templates,
            hypotheses,
            conclusion,
            proof,
        }
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    pub fn hypotheses(&self) -> &[Fact] {
        &self.hypotheses
    }

    pub fn conclusion(&self) -> FragmentId {
        self.conclusion
    }

    pub fn proof(&self) -> &UnresolvedProof {
        &self.proof
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnresolvedProof {
    Axiom,
    Theorem(ParseTreeId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Template {
    name: Ustr,
    cat: FormalSyntaxCatId,
    params: Vec<FormalSyntaxCatId>,
}

impl Template {
    pub fn new(name: Ustr, cat: FormalSyntaxCatId, params: Vec<FormalSyntaxCatId>) -> Self {
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
pub struct Fact {
    assumption: Option<FragmentId>,
    conclusion: FragmentId,
}

impl Fact {
    pub fn new(assumption: Option<FragmentId>, conclusion: FragmentId) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }

    pub fn assumption(&self) -> Option<FragmentId> {
        self.assumption
    }

    pub fn conclusion(&self) -> FragmentId {
        self.conclusion
    }
}

pub fn _debug_theorem_statement(id: TheoremId, stmt: &TheoremStatement, ctx: &crate::Ctx) {
    println!("Theorem {}:", id.name());
    for template in stmt.templates() {
        if template.params().is_empty() {
            println!(
                "  [{} : {}]",
                template.name(),
                ctx.formal_syntax[template.cat()].name(),
            );
        } else {
            println!(
                "  [{}({}) : {}]",
                template.name(),
                template
                    .params()
                    .iter()
                    .map(|cat| ctx.formal_syntax[*cat].name().as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                ctx.formal_syntax[template.cat()].name(),
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
