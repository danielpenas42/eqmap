/*!

  Timing analysis helpers for timing-aware optimization flows.

*/

use safety_net::Instantiable;
use safety_net::NetRef;
use safety_net::graph::CombDepthInfo;
use std::collections::HashSet;

#[derive(Debug)]
/// A representative critical path ending at a timing endpoint.
pub struct DelayPath<I: Instantiable> {
    /// The path from endpoint backward through critical fan-in.
    path: Vec<NetRef<I>>,
}

impl<I: Instantiable> DelayPath<I> {
    /// Returns the depth/length of the delay path.
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// The signal being driven by this path
    pub fn endpoint(&self) -> NetRef<I> {
        self.path[0].clone()
    }

    /// The nodes along the delay path as a slice
    pub fn path(&self) -> &[NetRef<I>] {
        &self.path
    }
    /// Expands and collects the transitive fan-in along the critical path provided by a branch factor of n.
    pub fn expand_n_nodes(&self, n: usize) -> HashSet<NetRef<I>> {
        let mut frontier: Vec<NetRef<I>> = self.path().to_vec();
        let mut expanded_nodes: HashSet<NetRef<I>> = frontier.iter().cloned().collect();

        for _ in 0..n {
            let mut next_frontier = Vec::new();

            for node in frontier {
                for driver in node.drivers().flatten() {
                    if expanded_nodes.insert(driver.clone()) {
                        next_frontier.push(driver);
                    }
                }
            }

            frontier = next_frontier;
        }

        expanded_nodes
    }
}

impl<I: Instantiable> IntoIterator for DelayPath<I> {
    type Item = NetRef<I>;
    type IntoIter = std::vec::IntoIter<NetRef<I>>;

    fn into_iter(self) -> Self::IntoIter {
        self.path.into_iter()
    }
}

fn build_path_from_endpoint<I: Instantiable>(
    analysis: &CombDepthInfo<'_, I>,
    endpoint: NetRef<I>,
) -> Option<DelayPath<I>> {
    let mut path = Vec::new();
    let mut current = endpoint;

    while let Some(crit) = analysis.get_crit_input(&current) {
        path.push(current.clone());
        if let Some(c) = crit.get_driver() {
            current = c.unwrap();
        } else {
            return None;
        }
    }

    path.push(current);
    Some(DelayPath { path })
}

/// Gets up to `n` most critical paths.
pub fn get_critical_paths<I: Instantiable>(
    analysis: &CombDepthInfo<'_, I>,
    n: usize,
) -> Vec<DelayPath<I>> {
    if analysis.get_max_depth().is_none() {
        return Vec::new();
    }

    let mut vec = Vec::new();

    for p in analysis.get_critical_points().into_iter().take(n) {
        if let Some(path) = build_path_from_endpoint(analysis, p.clone()) {
            vec.push(path);
        }
    }

    vec
}
