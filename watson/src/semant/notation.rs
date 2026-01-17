use crate::{
    generate_arena_handle,
    parse::{
        Span,
        parse_state::{Associativity, Precedence},
    },
    semant::formal_syntax::FormalSyntaxCatId,
};
use ustr::Ustr;

generate_arena_handle!(NotationPatternId<'ctx> => NotationPattern<'ctx>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotationPatternSource {
    UserDeclared(Span),
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotationPattern<'ctx> {
    name: Ustr,
    cat: FormalSyntaxCatId<'ctx>,
    parts: Vec<NotationPatternPart<'ctx>>,
    prec: Precedence,
    assoc: Associativity,
    source: NotationPatternSource,
    signature: NotationSignature<'ctx>,
}

impl<'ctx> NotationPattern<'ctx> {
    fn make_signature(
        cat: FormalSyntaxCatId<'ctx>,
        parts: &[NotationPatternPart<'ctx>],
    ) -> NotationSignature<'ctx> {
        let holes = parts
            .iter()
            .filter_map(|part| match part {
                NotationPatternPart::Cat(part) => {
                    let args = part.args().iter().map(|p| p.1).collect();
                    let hole = NotationSignatureHole::new(part.cat(), args);
                    Some(hole)
                }
                _ => None,
            })
            .collect();
        NotationSignature::new(cat, holes)
    }

    pub fn new(
        name: Ustr,
        cat: FormalSyntaxCatId<'ctx>,
        parts: Vec<NotationPatternPart<'ctx>>,
        prec: Precedence,
        assoc: Associativity,
        source: NotationPatternSource,
    ) -> Self {
        let signature = Self::make_signature(cat, &parts);

        Self {
            name,
            cat,
            parts,
            prec,
            assoc,
            source,
            signature,
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

    pub fn source(&self) -> NotationPatternSource {
        self.source
    }

    pub fn signature(&self) -> &NotationSignature<'ctx> {
        &self.signature
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotationPatternPart<'ctx> {
    Lit(Ustr),
    Kw(Ustr),
    Name,
    Cat(NotationPatternPartCat<'ctx>),
    Binding(FormalSyntaxCatId<'ctx>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotationPatternPartCat<'ctx> {
    /// The category of this child part.
    cat: FormalSyntaxCatId<'ctx>,
    /// List of arguments to this fragment. Each is the nth binding and its category.
    args: Vec<(usize, FormalSyntaxCatId<'ctx>)>,
}

impl<'ctx> NotationPatternPartCat<'ctx> {
    pub fn new(cat: FormalSyntaxCatId<'ctx>, args: Vec<(usize, FormalSyntaxCatId<'ctx>)>) -> Self {
        Self { cat, args }
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn args(&self) -> &[(usize, FormalSyntaxCatId<'ctx>)] {
        &self.args
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotationSignature<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    holes: Vec<NotationSignatureHole<'ctx>>,
}

impl<'ctx> NotationSignature<'ctx> {
    fn new(cat: FormalSyntaxCatId<'ctx>, holes: Vec<NotationSignatureHole<'ctx>>) -> Self {
        Self { cat, holes }
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn holes(&self) -> &[NotationSignatureHole<'ctx>] {
        &self.holes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotationSignatureHole<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    args: Vec<FormalSyntaxCatId<'ctx>>,
}

impl<'ctx> NotationSignatureHole<'ctx> {
    pub fn new(cat: FormalSyntaxCatId<'ctx>, args: Vec<FormalSyntaxCatId<'ctx>>) -> Self {
        Self { cat, args }
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn args(&self) -> &[FormalSyntaxCatId<'ctx>] {
        &self.args
    }
}

generate_arena_handle!(NotationBindingId<'ctx> => NotationBinding<'ctx>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NotationBinding<'ctx> {
    pattern: NotationPatternId<'ctx>,
    name_instantiations: Vec<Ustr>,
}

impl<'ctx> NotationBinding<'ctx> {
    pub fn new(pattern: NotationPatternId<'ctx>, name_instantiations: Vec<Ustr>) -> Self {
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
            NotationPatternPart::Cat(part_cat) => {
                out.push_str(&format!("<{}>", part_cat.cat().name()));
            }
            NotationPatternPart::Binding(binding) => {
                out.push_str(&format!("?{}", binding.name()));
            }
        }
    }
    out
}
