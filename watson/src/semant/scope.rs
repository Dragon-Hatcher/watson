use crate::{semant::{fragment::FragmentId, notation::NotationBindingId}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope<'ctx> {
    bindings: im::HashMap<NotationBindingId<'ctx>, Vec<ScopeEntry<'ctx>>>,
}

impl<'ctx> Scope<'ctx> {
    pub fn new() -> Self {
        Self {
            bindings: im::HashMap::new(),
        }
    }

    pub fn lookup(&self, binding: NotationBindingId<'ctx>) -> Option<&[ScopeEntry<'ctx>]> {
        self.bindings.get(&binding).map(|v| v.as_slice())
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
    frag: FragmentId<'ctx>,
    source: (), // TODO
}

impl<'ctx> ScopeEntry<'ctx> {
    pub fn new(frag: FragmentId<'ctx>) -> Self {
        Self { frag, source: () }
    }

    pub fn frag(&self) -> FragmentId<'ctx> {
        self.frag
    }
}