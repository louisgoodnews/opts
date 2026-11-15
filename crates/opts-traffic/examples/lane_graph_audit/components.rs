//! Weakly connected components of the lane graph (union-find over the successor links).

use std::collections::HashMap;

use opts_traffic::LaneGraph;

pub(crate) fn find(p: &mut [usize], x: usize) -> usize {
    let mut r = x;
    while p[r] != r {
        r = p[r];
    }
    let mut c = x;
    while p[c] != r {
        let next = p[c];
        p[c] = r;
        c = next;
    }
    r
}

/// Sizes of the weakly connected components, largest first.
pub(crate) fn component_sizes(graph: &LaneGraph) -> Vec<usize> {
    let n = graph.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for l in graph.lane_ids() {
        for s in graph.successors(l) {
            let (a, b) = (
                find(&mut parent, l.0 as usize),
                find(&mut parent, s.0 as usize),
            );
            parent[a] = b;
        }
    }
    let mut sizes: HashMap<usize, usize> = HashMap::new();
    for i in 0..n {
        *sizes.entry(find(&mut parent, i)).or_default() += 1;
    }
    let mut sizes: Vec<usize> = sizes.into_values().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    sizes
}
