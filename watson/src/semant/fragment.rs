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
}

impl<'ctx> Fragment<'ctx> {
    pub fn new(
        cat: FormalSyntaxCatId<'ctx>,
        head: FragHead<'ctx>,
        children: Vec<FragmentId<'ctx>>,
    ) -> Self {
        Self {
            cat,
            head,
            children,
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
