use std::collections::{HashSet, VecDeque};

use crate::{
    netlist::NetListInternal,
    graph::{fabric_graph::FabricGraph, node::NodeId},
};

pub fn validate(net_list: &NetListInternal, graph: &FabricGraph) -> Result<(), String> {
    let mut used_nodes_global: HashSet<NodeId> = HashSet::new();

    for tree in &net_list.plan {
        let result = tree
            .result
            .as_ref()
            .ok_or_else(|| format!("Tree {} has no SteinerTreeResult", tree.signal.name(graph)))?;

        // --- Check: no node is used in multiple signals ---
        for &n in &result.nodes {
            if !used_nodes_global.insert(n) {
                return Err(format!(
                    "Node {} is used by more than one signal (conflict at tree {})",
                    n.name(graph),
                    tree.signal.name(graph),
                ));
            }
        }

        // --- Reachability check: signal -> every sink using only result.nodes ---
        for &sink in &tree.sinks {
            if !is_reachable_within_set(graph, tree.signal, sink, &result.nodes) {
                println!("Sink in nodes: {}", result.nodes.contains(&sink));
                return Err(format!(
                    "sink {} is NOT reachable from signal {} using tree nodes",
                    sink.name(graph), tree.signal.name(graph),
                ));
            }
        }
    }

    Ok(())
}

/// BFS restricted to `allowed` node set.
fn is_reachable_within_set(graph: &FabricGraph, start: NodeId, target: NodeId, allowed: &HashSet<NodeId>) -> bool {
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
