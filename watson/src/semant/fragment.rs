use crate::{
    context::Ctx,
    generate_arena_handle,
    semant::{
        formal_syntax::{FormalSyntaxCatId, FormalSyntaxPatPart, FormalSyntaxRuleId},
        presentation::{Pres, PresFrag, PresHead},
        theorems::PresFact,
    },
};

generate_arena_handle! { FragmentId<'ctx> => Fragment<'ctx> }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fragment<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    head: FragHead<'ctx>,
    children: Vec<FragmentId<'ctx>>,

    // flags for efficient search:
    has_hole: bool,
    has_template: bool,
    unclosed_count: usize,
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

        if matches!(head, FragHead::Hole(_) | FragHead::Variable(_, _)) {
            assert!(children.is_empty());
        }

        let children_unclosed = children
            .iter()
            .map(|c| c.unclosed_count())
            .max()
            .unwrap_or(0);
        let unclosed_count = match head {
            FragHead::RuleApplication(rule_app) => {
                // The rule application adds a certain number of bindings which
                // closes that many of the unclosed bindings from the children.
                children_unclosed.saturating_sub(rule_app.bindings_added)
            }
            FragHead::Variable(_cat, db_idx) => db_idx.max(children_unclosed),
            _ => children_unclosed,
        };

        Self {
            cat,
            head,
            children,
            has_hole,
            has_template,
            unclosed_count,
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

    pub fn has_template(&self) -> bool {
        self.has_template
    }

    pub fn unclosed_count(&self) -> usize {
        self.unclosed_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FragHead<'ctx> {
    RuleApplication(FragRuleApplication<'ctx>),
    Variable(FormalSyntaxCatId<'ctx>, usize), // Debruijn index
    TemplateRef(usize),
    Hole(usize),
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
    ctx: &Ctx<'ctx>,
) -> PresFrag<'ctx> {
    let frag = Fragment::new(cat, FragHead::Hole(idx), Vec::new());
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
            out.push('(');

            let mut child_idx = 0;
            for (i, part) in rule.rule().pattern().parts().iter().enumerate() {
                if i != 0 {
                    out.push(' ');
                }
                match part {
                    FormalSyntaxPatPart::Cat(_) => {
                        let child = &frag.children()[child_idx];
                        out.push_str(&_debug_fragment(*child));
                        child_idx += 1;
                    }
                    FormalSyntaxPatPart::Binding(_) => out.push_str("??"),
                    FormalSyntaxPatPart::Lit(str) => out.push_str(str),
                }
            }

            out.push(')');
            out
        }
        FragHead::Variable(_cat, idx) => format!("?{}", idx),
        FragHead::TemplateRef(idx) => format!("${}", idx),
        FragHead::Hole(idx) => format!("_{}", idx),
    }
}
