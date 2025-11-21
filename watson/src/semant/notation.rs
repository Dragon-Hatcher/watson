use crate::{
    generate_arena_handle,
    parse::parse_state::{Associativity, Precedence},
    semant::formal_syntax::FormalSyntaxCatId,
};
use ustr::Ustr;

generate_arena_handle!(NotationPatternId<'ctx> => NotationPattern<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotationPattern<'ctx> {
    name: Ustr,
    cat: FormalSyntaxCatId<'ctx>,
    parts: Vec<NotationPatternPart<'ctx>>,
    prec: Precedence,
    assoc: Associativity,
}

impl<'ctx> NotationPattern<'ctx> {
    pub fn new(
        name: Ustr,
        cat: FormalSyntaxCatId<'ctx>,
        parts: Vec<NotationPatternPart<'ctx>>,
        prec: Precedence,
        assoc: Associativity,
    ) -> Self {
        Self {
            name,
            cat,
            parts,
            prec,
            assoc,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn parts(&self) -> &[NotationPatternPart<'ctx>] {
        &self.parts
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn prec(&self) -> Precedence {
        self.prec
    }

    pub fn assoc(&self) -> Associativity {
        self.assoc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotationPatternPart<'ctx> {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Cat(FormalSyntaxCatId<'ctx>),
    Binding(FormalSyntaxCatId<'ctx>),
}

generate_arena_handle!(NotationBindingId<'ctx> => NotationBinding<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotationBinding<'ctx> {
    pattern: NotationPatternId<'ctx>,
    instantiations: Vec<NotationInstantiationPart>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationInstantiationPart {
    Name(Ustr),
}