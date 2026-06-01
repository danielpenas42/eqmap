/*!

  Timing analysis helpers for timing-aware optimization flows.

*/

use crate::netlist::PrimitiveCell;
use safety_net::NetRef;
use safety_net::graph::CombDepthInfo;
use std::collections::HashSet;

#[derive(Debug)]
/// A representative critical path ending at a timing endpoint.
pub struct DelayPath {
    /// The path from endpoint backward through critical fan-in.
    path: Vec<NetRef<PrimitiveCell>>,
}

impl DelayPath {
    /// Returns the depth/length of the delay path.
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// The signal being driven by this path
    pub fn endpoint(&self) -> NetRef<PrimitiveCell> {
        self.path[0].clone()
    }

    /// The nodes along the delay path as a slice
    pub fn path(&self) -> &[NetRef<PrimitiveCell>] {
        &self.path
    }
}

impl IntoIterator for DelayPath {
    type Item = NetRef<PrimitiveCell>;
    type IntoIter = std::vec::IntoIter<NetRef<PrimitiveCell>>;

    fn into_iter(self) -> Self::IntoIter {
        self.path.into_iter()
    }
}

fn build_path_from_endpoint(
    analysis: &CombDepthInfo<'_, PrimitiveCell>,
    endpoint: NetRef<PrimitiveCell>,
) -> Option<DelayPath> {
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

/// Gets one of the top critical paths from the combinational-depth analysis.
pub fn get_critical_path(analysis: &CombDepthInfo<'_, PrimitiveCell>) -> Option<DelayPath> {
    analysis.get_max_depth()?;
    let endpoint = analysis.get_critical_points().into_iter().next()?.clone();
    build_path_from_endpoint(analysis, endpoint)
}

/// Gets up to `n` most critical paths.
pub fn get_critical_paths(analysis: &CombDepthInfo<'_, PrimitiveCell>, n: usize) -> Vec<DelayPath> {
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

/// Expands a critical path backward through fan-in for `n` frontier steps.
pub fn expand_n_nodes(path: DelayPath, n: usize) -> HashSet<NetRef<PrimitiveCell>> {
    let mut frontier: Vec<NetRef<PrimitiveCell>> = path.into_iter().collect();
    let mut expanded_nodes: HashSet<NetRef<PrimitiveCell>> = frontier.iter().cloned().collect();

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
