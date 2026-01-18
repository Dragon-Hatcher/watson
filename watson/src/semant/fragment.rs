use crate::{
    context::Ctx,
    generate_arena_handle,
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
        presentation::{Pres, PresFrag, PresHead},
        theorems::PresFact,
    },
};
use itertools::Itertools;

generate_arena_handle! { FragmentId<'ctx> => Fragment<'ctx> }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fragment<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    head: FragHead<'ctx>,
    children: Vec<FragmentId<'ctx>>,

    // flags for efficient search:
    has_hole: bool,
    has_var_hole: bool,
    has_template: bool,
    unclosed_vars: usize,
}

impl<'ctx> Fragment<'ctx> {
    pub fn new(
        cat: FormalSyntaxCatId<'ctx>,
        head: FragHead<'ctx>,
        children: Vec<FragmentId<'ctx>>,
    ) -> Self {
        let has_template =
            matches!(head, FragHead::TemplateRef(_)) || children.iter().any(|c| c.has_template);
        let has_hole = matches!(head, FragHead::Hole(_)) || children.iter().any(|c| c.has_hole);
        let has_var_hole =
            matches!(head, FragHead::VarHole(_)) || children.iter().any(|c| c.has_var_hole);
        let unclosed_vars = match head {
            FragHead::Var(idx) => idx + 1, // zero indexed
            _ => {
                // The max unclosed var of our children
                let children = children
                    .iter()
                    .map(|c| c.unclosed_vars())
                    .max()
                    .unwrap_or(0);
                // Minus however many bindings we introduced here
                let bindings = head.bindings_added();
                children.saturating_sub(bindings)
            }
        };

        if matches!(head, FragHead::Var(_) | FragHead::VarHole(_)) {
            assert!(children.is_empty());
        }

        Self {
            cat,
            head,
            children,
            has_hole,
            has_var_hole,
            has_template,
            unclosed_vars,
        }
    }

    pub fn cat(&self) -> FormalSyntaxCatId<'ctx> {
        self.cat
    }

    pub fn head(&self) -> FragHead<'ctx> {
        self.head
    }

    pub fn children(&self) -> &[FragmentId<'ctx>] {
        &self.children
    }

    pub fn has_hole(&self) -> bool {
        self.has_hole
    }

    pub fn has_var_hole(&self) -> bool {
        self.has_var_hole
    }

    pub fn has_template(&self) -> bool {
        self.has_template
    }

    pub fn unclosed_vars(&self) -> usize {
        self.unclosed_vars
    }

    pub fn is_closed(&self) -> bool {
        self.unclosed_vars() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FragHead<'ctx> {
    RuleApplication(FragRuleApplication<'ctx>),
    Var(usize),
    TemplateRef(usize),
    Hole(usize),
    VarHole(usize),
}

impl<'ctx> FragHead<'ctx> {
    pub fn bindings_added(&self) -> usize {
        match self {
            FragHead::RuleApplication(app) => app.bindings_added(),
            FragHead::Var(_) => 0,
            FragHead::TemplateRef(_) => 0,
            FragHead::Hole(_) => 0,
            FragHead::VarHole(_) => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FragRuleApplication<'ctx> {
    rule: FormalSyntaxRuleId<'ctx>,
    bindings_added: usize,
}

impl<'ctx> FragRuleApplication<'ctx> {
    pub fn new(rule: FormalSyntaxRuleId<'ctx>, bindings_added: usize) -> Self {
        Self {
            rule,
            bindings_added,
        }
    }

    pub fn rule(&self) -> FormalSyntaxRuleId<'ctx> {
        self.rule
    }

    pub fn bindings_added(&self) -> usize {
        self.bindings_added
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

pub fn hole_frag<'ctx>(
    idx: usize,
    cat: FormalSyntaxCatId<'ctx>,
    children: Vec<PresFrag<'ctx>>,
    ctx: &Ctx<'ctx>,
) -> PresFrag<'ctx> {
    let frag_children = children.iter().map(|c| c.frag()).collect();
    let frag = Fragment::new(cat, FragHead::Hole(idx), frag_children);
    let frag = ctx.arenas.fragments.intern(frag);
    let pres = Pres::new(PresHead::FormalFrag(frag.head()), children);
    let pres = ctx.arenas.presentations.intern(pres);

    // The presentation is already formal so we can pass the pres as the
    // formal pres.
    PresFrag::new(frag, pres, pres)
}

pub fn var_frag<'ctx>(idx: usize, cat: FormalSyntaxCatId<'ctx>, ctx: &Ctx<'ctx>) -> PresFrag<'ctx> {
    let frag = Fragment::new(cat, FragHead::Var(idx), Vec::new());
    let frag = ctx.arenas.fragments.intern(frag);
    let pres = Pres::new(PresHead::FormalFrag(frag.head()), Vec::new());
    let pres = ctx.arenas.presentations.intern(pres);

    // The presentation is already formal so we can pass the pres as the
    // formal pres.
    PresFrag::new(frag, pres, pres)
}

pub fn _debug_fact<'ctx>(fact: &PresFact<'ctx>) -> String {
    let conclusion = _debug_fragment(fact.conclusion().frag());
    match fact.assumption() {
        Some(assumption) => format!("{} |- {}", _debug_fragment(assumption.frag()), conclusion),
        None => conclusion,
    }
}

pub fn _debug_fragment<'ctx>(frag: FragmentId<'ctx>) -> String {
    match frag.head() {
        FragHead::RuleApplication(rule) => {
            let mut out = String::new();

            let mut child_idx = 0;
            for part in rule.rule().pattern().parts() {
                match part {
                    FormalSyntaxPatPart::Cat(_) => {
                        let child = &frag.children()[child_idx];
                        out.push_str(&_debug_fragment(*child));
                        child_idx += 1;
                    }
                    FormalSyntaxPatPart::Binding(_) => out.push_str("?"),
                    FormalSyntaxPatPart::Lit(str) => out.push_str(str),
                }
            }

            out
        }
        FragHead::Var(idx) => format!("'{}", idx),
        FragHead::TemplateRef(idx) => {
            if frag.children().len() > 0 {
                let children = frag
                    .children()
                    .iter()
                    .map(|&c| _debug_fragment(c))
                    .join(", ");
                format!("${}({})", idx, children)
            } else {
                format!("${}", idx)
            }
        }
        FragHead::Hole(idx) => {
            if frag.children().len() > 0 {
                let children = frag
                    .children()
                    .iter()
                    .map(|&c| _debug_fragment(c))
                    .join(", ");
                format!("_{}({})", idx, children)
            } else {
                format!("_{}", idx)
            }
        }
        FragHead::VarHole(idx) => format!("\"{}", idx),
    }
}
