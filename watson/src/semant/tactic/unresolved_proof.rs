use crate::semant::custom_grammar::inst::CustomGrammarInst;

pub enum UnresolvedProof<'ctx> {
    Axiom,
    Theorem(CustomGrammarInst<'ctx>),
}
