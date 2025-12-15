use crate::{
    generate_arena_handle,
    semant::{
        formal_syntax::FormalSyntaxPatPart,
        fragment::{FragHead, FragmentId},
        notation::{NotationBindingId, NotationPatternPart},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresFrag<'ctx> {
    /// The formal fragment this is a presentation of.
    frag: FragmentId<'ctx>,
    /// The presentation of the fragment.
    pres: PresId<'ctx>,
    /// The presentation for the same fragment but with all user defined notation
    /// replaced with formal syntax.
    formal: PresId<'ctx>,
}

impl<'ctx> PresFrag<'ctx> {
    pub fn new(frag: FragmentId<'ctx>, pres: PresId<'ctx>, formal: PresId<'ctx>) -> Self {
        Self { frag, pres, formal }
    }

    pub fn frag(&self) -> FragmentId<'ctx> {
        self.frag
    }

    pub fn pres(&self) -> PresId<'ctx> {
        self.pres
    }

    pub fn formal(&self) -> Self {
        // Since the formal presentation already contains only formal nodes,
        // the formal presentation of that tree will be the same.
        Self {
            frag: self.frag,
            pres: self.formal,
            formal: self.formal,
        }
    }

    pub fn formal_pres(&self) -> PresId<'ctx> {
        self.formal
    }

    pub fn print(&self) -> String {
        self.pres().print()
    }
}

generate_arena_handle! {PresId<'ctx> => Pres<'ctx>}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pres<'ctx> {
    /// The node value in the presentation tree.
    head: PresHead<'ctx>,
    /// The children in the presentation tree.
    children: Vec<PresFrag<'ctx>>,
}

impl<'ctx> Pres<'ctx> {
    pub fn new(head: PresHead<'ctx>, children: Vec<PresFrag<'ctx>>) -> Self {
        Self { head, children }
    }

    pub fn head(&self) -> PresHead<'ctx> {
        self.head
    }

    pub fn children(&self) -> &[PresFrag<'ctx>] {
        &self.children
    }

    pub fn print(&self) -> String {
        match self.head() {
            PresHead::FormalFrag(FragHead::Hole(idx)) => format!("_{idx}"),
            PresHead::FormalFrag(FragHead::TemplateRef(idx)) => format!("#{idx}"),
            PresHead::FormalFrag(FragHead::Variable(_, _)) => todo!(),
            PresHead::FormalFrag(FragHead::RuleApplication(rule_app)) => {
                let mut out = String::new();
                let mut children = self.children().iter();

                for (i, part) in rule_app._rule().pattern().parts().iter().enumerate() {
                    use FormalSyntaxPatPart as P;

                    if i != 0 {
                        out.push(' ');
                    }

                    match part {
                        P::Lit(lit) => out.push_str(lit),
                        P::Binding(_) => todo!(),
                        P::Cat(_) => out.push_str(&children.next().unwrap().print()),
                    }
                }

                out
            }
            PresHead::Notation(binding, _) => {
                let mut out = String::new();
                let mut children = self.children().iter();
                let mut name_instantiations = binding.name_instantiations().iter();

                for (i, part) in binding.pattern().parts().iter().enumerate() {
                    use NotationPatternPart as P;

                    if i != 0 {
                        out.push(' ');
                    }

                    match part {
                        P::Lit(lit) => out.push_str(lit),
                        P::Kw(kw) => out.push_str(kw),
                        P::Name => out.push_str(name_instantiations.next().unwrap()),
                        P::Cat(_) => out.push_str(&children.next().unwrap().print()),
                        P::Binding(_) => todo!(),
                    }
                }

                out
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresHead<'ctx> {
    /// The notation for the fragment is directly a formal syntax fragment.
    FormalFrag(FragHead<'ctx>),
    /// The notation for the fragment is a notation binding which is replaced
    /// by the given PresFrag when instantiated.
    Notation(NotationBindingId<'ctx>, PresFrag<'ctx>),
}
