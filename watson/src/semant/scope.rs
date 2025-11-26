use crate::{semant::{fragment::FragmentId, notation::NotationBindingId}};

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
        self.bindings.get(&binding).map(|v| v.as_slice().last().unwrap())
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
    /// How many bindings exist above the fragment.
    binding_depth: usize,
    source: (), // TODO
}

impl<'ctx> ScopeEntry<'ctx> {
    pub fn new(frag: FragmentId<'ctx>) -> Self {
        Self::new_with_depth(frag, 0)
    }

    pub fn new_with_depth(frag: FragmentId<'ctx>, binding_depth: usize) -> Self {
        Self { frag, binding_depth, source: () }
    }

    pub fn frag(&self) -> FragmentId<'ctx> {
        self.frag
    }

    pub fn binding_depth(&self) -> usize {
        self.binding_depth
    }
}