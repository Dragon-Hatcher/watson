use rustc_hash::{FxHashMap, FxHashSet};

use crate::semant::{proof_status::ProofStatuses, theorems::TheoremId};

pub fn find_circular_dependency_groups(statuses: &ProofStatuses) -> Vec<Vec<TheoremId>> {
    // To check for circularity, we find the strongly connected components of the
    // theorem dependency graph.

    let mut index = 0;
    let mut indices: FxHashMap<TheoremId, usize> = FxHashMap::default();
    let mut lowlinks: FxHashMap<TheoremId, usize> = FxHashMap::default();
    let mut stack: Vec<TheoremId> = Vec::new();
    let mut on_stack: FxHashSet<TheoremId> = FxHashSet::default();
    let mut visited: FxHashSet<TheoremId> = FxHashSet::default();
    let mut sccs: Vec<Vec<TheoremId>> = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn dfs(
        at: TheoremId,
        visited: &mut FxHashSet<TheoremId>,
        stack: &mut Vec<TheoremId>,
        on_stack: &mut FxHashSet<TheoremId>,
        indices: &mut FxHashMap<TheoremId, usize>,
        lowlinks: &mut FxHashMap<TheoremId, usize>,
        index: &mut usize,
        sccs: &mut Vec<Vec<TheoremId>>,
        statuses: &ProofStatuses,
    ) {
        // Place the current theorem on the stack
        stack.push(at);
        on_stack.insert(at);

        // Give the theorem an index and assign its lowlink to that index
        visited.insert(at);
        indices.insert(at, *index);
        lowlinks.insert(at, *index);
        *index += 1;

        for to in statuses[at].theorems_used() {
            if !visited.contains(to) {
                dfs(
                    *to, visited, stack, on_stack, indices, lowlinks, index, sccs, statuses,
                );
            }

            if on_stack.contains(to) {
                lowlinks.insert(at, lowlinks[&at].min(lowlinks[to]));
            }
        }

        // If our index is equal to our lowlink, we found a strongly connected component
        if indices[&at] == lowlinks[&at] {
            // Pop nodes of the stack until we reach the current node
            let mut scc = Vec::new();
            while let Some(node) = stack.pop() {
                on_stack.remove(&node);
                scc.push(node);
                if node == at {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    for (&theorem_id, _status) in statuses.iter() {
        if visited.contains(&theorem_id) {
            continue;
        }

        dfs(
            theorem_id,
            &mut visited,
            &mut stack,
            &mut on_stack,
            &mut indices,
            &mut lowlinks,
            &mut index,
            &mut sccs,
            statuses,
        );
    }

    sccs.retain(|scc| {
        scc.len() > 1 || {
            let id = scc[0];
            statuses[id].theorems_used().contains(&id)
        }
    });

    sccs
}
