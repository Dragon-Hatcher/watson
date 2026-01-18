use crate::semant::{
    formal_syntax::FormalSyntaxCatId, notation::NotationBindingId, presentation::PresFrag,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope<'ctx> {
    // TODO
    bindings: im::HashMap<NotationBindingId<'ctx>, Vec<ScopeEntry<'ctx>>>,
}

impl<'ctx> Scope<'ctx> {
    pub fn new() -> Self {
        Self {
            bindings: im::HashMap::new(),
        }
    }

    pub fn lookup(&self, binding: NotationBindingId<'ctx>) -> Option<&ScopeEntry<'ctx>> {
        self.bindings
            .get(&binding)
            .map(|v| v.as_slice().last().unwrap())
    }

    pub fn child_with(&self, binding: NotationBindingId<'ctx>, entry: ScopeEntry<'ctx>) -> Self {
        let mut new_bindings = self.bindings.clone();
        new_bindings
            .entry(binding)
            .and_modify(|entries| entries.push(entry))
            .or_insert_with(|| vec![entry]);
        Self {
            bindings: new_bindings,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeEntry<'ctx> {
    replacement: ScopeReplacement<'ctx>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeReplacement<'ctx> {
    Frag(PresFrag<'ctx>),
    Hole(FormalSyntaxCatId<'ctx>, usize),
}

impl<'ctx> ScopeEntry<'ctx> {
    pub fn new(frag: PresFrag<'ctx>) -> Self {
        Self {
            replacement: ScopeReplacement::Frag(frag),
        }
    }

    pub fn new_hole(cat: FormalSyntaxCatId<'ctx>, idx: usize) -> Self {
        Self {
            replacement: ScopeReplacement::Hole(cat, idx),
        }
    }

    pub fn replacement(&self) -> ScopeReplacement<'ctx> {
        self.replacement
    }
}
