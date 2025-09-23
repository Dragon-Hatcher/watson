use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::semant::{
    formal_syntax::FormalSyntaxCatId,
    fragment::{_debug_fragment, FragmentId},
};

pub struct TheoremStatements {
    theorems: FxHashMap<Ustr, TheoremStatement>,
}

impl TheoremStatements {
    pub fn new() -> Self {
        Self {
            theorems: FxHashMap::default(),
        }
    }

    pub fn add(&mut self, name: Ustr, statement: TheoremStatement) {
        self.theorems.insert(name, statement);
    }

    pub fn get(&self, name: Ustr) -> Option<&TheoremStatement> {
        self.theorems.get(&name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ustr, &TheoremStatement)> {
        self.theorems.iter()
    }
}

#[derive(Debug, Clone)]
pub struct TheoremStatement {
    templates: FxHashMap<Ustr, Template>,
    hypotheses: Vec<Fact>,
    conclusion: FragmentId,
}

impl TheoremStatement {
    pub fn new(
        templates: FxHashMap<Ustr, Template>,
        hypotheses: Vec<Fact>,
        conclusion: FragmentId,
    ) -> Self {
        Self {
            templates,
            hypotheses,
            conclusion,
        }
    }

    pub fn templates(&self) -> &FxHashMap<Ustr, Template> {
        &self.templates
    }

    pub fn hypotheses(&self) -> &[Fact] {
        &self.hypotheses
    }

    pub fn conclusion(&self) -> FragmentId {
        self.conclusion
    }
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

pub fn _debug_theorem_statement(name: Ustr, stmt: &TheoremStatement, ctx: &crate::Ctx) {
    println!("Theorem {name}:");
    for (name, template) in stmt.templates() {
        if template.params().is_empty() {
            println!(
                "  [{} : {}]",
                name,
                ctx.formal_syntax[template.cat()].name(),
            );
        } else {
            println!(
                "  [{}({}) : {}]",
                name,
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
