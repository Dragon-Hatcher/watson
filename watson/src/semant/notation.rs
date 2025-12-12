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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotationBinding<'ctx> {
    pattern: NotationPatternId<'ctx>,
    name_instantiations: Vec<Ustr>,
}

impl<'ctx> NotationBinding<'ctx> {
    pub fn new(
        pattern: NotationPatternId<'ctx>,
        name_instantiations: Vec<Ustr>,
    ) -> Self {
        Self {
            pattern,
            name_instantiations,
        }
    }

    pub fn pattern(&self) -> NotationPatternId<'ctx> {
        self.pattern
    }

    pub fn name_instantiations(&self) -> &[Ustr] {
        &self.name_instantiations
    }
}

pub fn _debug_binding<'ctx>(binding: NotationBindingId<'ctx>) -> String {
    let mut out = String::new();
    let mut names = 0;
    for (i, part) in binding.pattern().parts().iter().enumerate() {
        if i != 0 {
            out.push(' ');
        }
        match part {
            NotationPatternPart::Lit(lit) => {
                out.push_str(lit.as_str());
            }
            NotationPatternPart::Kw(kw) => {
                out.push_str(kw.as_str());
            }
            NotationPatternPart::Name => {
                let name = binding.name_instantiations()[names];
                out.push_str(name.as_str());
                names += 1;
            }
            NotationPatternPart::Cat(cat) => {
                out.push_str(&format!("<{}>", cat.name()));
            }
            NotationPatternPart::Binding(binding) => {
                out.push_str(&format!("?{}", binding.name()));
            }
        }
    }
    out
}
