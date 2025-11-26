use crate::{
    generate_arena_handle,
    semant::formal_syntax::{FormalSyntaxCatId, FormalSyntaxRuleId},
};

generate_arena_handle! { FragmentId<'ctx> => Fragment<'ctx> }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fragment<'ctx> {
    cat: FormalSyntaxCatId<'ctx>,
    head: FragHead<'ctx>,
    children: Vec<FragmentId<'ctx>>,

    // flags for efficient search:
    has_hole: bool,
    unclosed_count: usize,
}

impl<'ctx> Fragment<'ctx> {
    pub fn new(
        cat: FormalSyntaxCatId<'ctx>,
        head: FragHead<'ctx>,
        children: Vec<FragmentId<'ctx>>,
    ) -> Self {
        let has_hole = matches!(head, FragHead::Hole(_)) || children.iter().any(|c| c.has_hole);

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
