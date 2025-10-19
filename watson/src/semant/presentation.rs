use crate::generate_arena_handle;
use ustr::Ustr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresentationTree<'ctx> {
    pres: PresentationId<'ctx>,
    data: PresTreeData<'ctx>,
}

impl<'ctx> PresentationTree<'ctx> {
    pub fn new(pres: PresentationId<'ctx>, data: PresTreeData<'ctx>) -> Self {
        Self { pres, data }
    }

    pub fn render_str(&self) -> String {
        self.pres.render_str(self.data())
    }

    pub fn pres(&self) -> PresentationId<'ctx> {
        self.pres
    }

    pub fn data(&self) -> &PresTreeData<'ctx> {
        &self.data
    }
}

generate_arena_handle! { PresentationTreeId<'ctx> => PresentationTree<'ctx> }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactPresentation<'ctx> {
    assumption: Option<PresentationTreeId<'ctx>>,
    conclusion: PresentationTreeId<'ctx>,
}

impl<'ctx> FactPresentation<'ctx> {
    pub fn render_str(&self) -> String {
        if let Some(assumption) = self.assumption {
            format!(
                "assume {} |- {}",
                assumption.render_str(),
                self.conclusion.render_str()
            )
        } else {
            self.conclusion.render_str()
        }
    }

    pub fn assumption(&self) -> Option<PresentationTreeId<'ctx>> {
        self.assumption
    }

    pub fn conclusion(&self) -> PresentationTreeId<'ctx> {
        self.conclusion
    }
}

impl<'ctx> FactPresentation<'ctx> {
    pub fn new(
        assumption: Option<PresentationTreeId<'ctx>>,
        conclusion: PresentationTreeId<'ctx>,
    ) -> Self {
        Self {
            assumption,
            conclusion,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresTreeData<'ctx> {
    Rule(PresTreeRuleApp<'ctx>),
    Template(PresTreeTemplate<'ctx>),
    Hole,
}

impl<'ctx> PresTreeData<'ctx> {
    pub fn child_on_path(&self, path: &[usize]) -> PresentationTreeId<'ctx> {
        let next = match path {
            [idx] | [idx, ..] => match self {
                PresTreeData::Rule(rule) => {
                    let child = rule.children()[*idx];
                    let PresTreeChild::Fragment(tree) = child else {
                        unreachable!();
                    };
                    tree
                }
                PresTreeData::Template(temp) => temp.args()[*idx],
                PresTreeData::Hole => unreachable!(),
            },
            _ => todo!(),
        };

        match path {
            [_one] => next,
            [_one, rest @ ..] => next.data().child_on_path(rest),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresTreeRuleApp<'ctx> {
    children: Vec<PresTreeChild<'ctx>>,
}

impl<'ctx> PresTreeRuleApp<'ctx> {
    pub fn new(children: Vec<PresTreeChild<'ctx>>) -> Self {
        Self { children }
    }

    pub fn children(&self) -> &[PresTreeChild<'ctx>] {
        &self.children
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresTreeTemplate<'ctx> {
    args: Vec<PresentationTreeId<'ctx>>,
}

impl<'ctx> PresTreeTemplate<'ctx> {
    pub fn new(args: Vec<PresentationTreeId<'ctx>>) -> Self {
        Self { args }
    }

    pub fn args(&self) -> &[PresentationTreeId<'ctx>] {
        &self.args
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresTreeChild<'ctx> {
    Fragment(PresentationTreeId<'ctx>),
    Variable,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Presentation<'ctx> {
    Rule(PresRuleApplication),
    Template(PresTemplate<'ctx>),
    Hole(usize),
}

impl<'ctx> Presentation<'ctx> {
    pub fn render_str(&self, tree: &PresTreeData) -> String {
        match self {
            Presentation::Rule(rule_app) => rule_app.render_str(tree),
            Presentation::Template(temp) => temp.render_str(tree),
            Presentation::Hole(idx) => format!("_{idx}"),
        }
    }
}

generate_arena_handle! { PresentationId<'ctx> => Presentation<'ctx> }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresRuleApplication {
    parts: Vec<PresPart>,
}

impl PresRuleApplication {
    pub fn new(parts: Vec<PresPart>) -> Self {
        Self { parts }
    }

    pub fn parts(&self) -> &[PresPart] {
        &self.parts
    }

    pub fn render_str(&self, tree: &PresTreeData) -> String {
        let mut str = String::new();
        for (i, part) in self.parts.iter().enumerate() {
            if i != 0 {
                str += " ";
            }

            match part {
                PresPart::Str(lit) => str += lit,
                PresPart::Binding(name) | PresPart::Variable(name) => str += name,
                PresPart::Subpart(path) => {
                    let target = tree.child_on_path(path);
                    str += &target.render_str();
                }
            }
        }
        str
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresPart {
    Str(Ustr),
    Binding(Ustr),
    Variable(Ustr),
    Subpart(Vec<usize>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresTemplate<'ctx> {
    name: Ustr,
    args: Vec<PresentationId<'ctx>>,
}

impl<'ctx> PresTemplate<'ctx> {
    pub fn new(name: Ustr, args: Vec<PresentationId<'ctx>>) -> Self {
        Self { name, args }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn render_str(&self, tree: &PresTreeData) -> String {
        let mut str = String::new();
        str += &self.name;

        if !self.args.is_empty() {
            str += "(";
            for (i, arg) in self.args.iter().enumerate() {
                if i != 0 {
                    str += ", "
                }
                str += &arg.render_str(tree);
            }
            str += ")"
        }

        str
    }
}
