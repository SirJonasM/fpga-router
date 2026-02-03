use std::collections::{HashSet, VecDeque};

use crate::fabric_graph::{FabricGraph, Routing};

pub fn validate(route_plan: &[Routing], graph: &FabricGraph )-> Result<(),String> {
    let mut used_nodes_global: HashSet<usize> = HashSet::new();
    let node_count = graph.nodes.len();

    for (tree_idx, tree) in route_plan.iter().enumerate() {
        let result = tree
            .result
            .as_ref()
            .ok_or_else(|| format!("Tree {} has no SteinerTreeResult", tree_idx))?;

        // --- Check: all nodes exist ---
        for &n in &result.nodes {
            if n >= node_count {
                return Err(format!("Tree {} contains invalid node index {} (out of range)", tree_idx, n));
            }
        }

        // --- Check: no node is used in multiple signals ---
        for &n in &result.nodes {
            if !used_nodes_global.insert(n) {
                return Err(format!(
                    "Node {n} is used by more than one signal (conflict at tree {tree_idx})",
                ));
            }
        }

        // --- Check: signal exists ---
        if tree.signal >= node_count {
            return Err(format!("Tree {} uses invalid signal node {}", tree_idx, tree.signal));
        }

        // --- Reachability check: signal -> every sink using only result.nodes ---
        for &sink in &tree.sinks {
            if sink >= node_count {
                return Err(format!("Tree {tree_idx} has invalid sink {sink}"));
            }

            if !is_reachable_within_set(graph, tree.signal, sink, &result.nodes) {
                println!("Sink in nodes: {}", result.nodes.contains(&sink));
                return Err(format!(
                    "Tree {}: sink {} is NOT reachable from signal {} using tree nodes",
                    tree_idx, sink, tree.signal,
                ));
            }
        }
    }

    Ok(())
}

/// BFS restricted to `allowed` node set.
fn is_reachable_within_set(graph: &FabricGraph, start: usize, target: usize, allowed: &HashSet<usize>) -> bool {
    if start == target {
        return true;
    }
    if !allowed.contains(&start) || !allowed.contains(&target) {
        return false;
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(u) = queue.pop_front() {
        for edge in &graph.map[u] {
            let v = edge.node_id;

            if !allowed.contains(&v) {
                continue;
            }
            if visited.insert(v) {
                if v == target {
                    return true;
                }
                queue.push_back(v);
            }
        }
    }
    false
}
