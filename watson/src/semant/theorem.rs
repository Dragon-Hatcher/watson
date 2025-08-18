use crate::semant::formal_syntax::FormalSyntaxCatId;
use ustr::Ustr;

pub struct Theorem {
    name: Ustr,
    templates: Vec<Template>,
    hypotheses: Vec<Sentence>,
    statement: Sentence,
    proof: Proof,
}

pub enum Proof {
    Axiom,
    Theorem,
}

struct Sentence;

struct Template {
    name: Ustr,
    cat: FormalSyntaxCatId,
    args: Vec<FormalSyntaxCatId>,
}
