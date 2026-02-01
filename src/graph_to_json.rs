use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::fabric_graph::FabricGraph;
use crate::Routing;


#[derive(Serialize, Debug, )]
struct JsonNode {
    id: usize,
    label: String,
    usage: u32,
    signals: HashSet<usize>,
}

#[derive(Serialize, Debug, )]
struct JsonEdge {
    from: usize,
    to: usize,
    weight: f32,
    signals: HashSet<usize>,
}

#[derive(Serialize, Debug, )]
pub struct JsonGraph {
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

pub fn export_steiner_to_json(fg: &FabricGraph, steiner: &[Routing]) -> JsonGraph{
    let mut nodes: Vec<JsonNode> = Vec::new();
    let mut edges: Vec<JsonEdge> = Vec::new();

    let mut node_map: HashMap<usize, usize> = HashMap::new();
    let mut edge_map: HashMap<(usize, usize), usize> = HashMap::new();

    for tree in steiner {
        let signal_id = tree.signal;

        let result = match &tree.result {
            Some(r) => r,
            None => continue,
        };

        for &node_id in &result.nodes {
            add_json_node(fg, &mut nodes, &mut node_map, node_id, signal_id);
        }

        for &u in &result.nodes {
            for edge in &fg.map[u] {
                let v = edge.node_id;
                if result.nodes.contains(&v) {
                    add_json_edge(fg, &mut edges, &mut edge_map, u, v, signal_id);
                }
            }
        }

        for path in result.paths.values() {
            for pair in path.windows(2) {
                let u = pair[0];
                let v = pair[1];
                add_json_edge(fg, &mut edges, &mut edge_map, u, v, signal_id);
            }
        }
    }

    JsonGraph { nodes, edges }

}

/// Adds or updates a JSON node
fn add_json_node(
    fg: &FabricGraph,
    nodes: &mut Vec<JsonNode>,
    node_map: &mut HashMap<usize, usize>,
    node_id: usize,
    signal_id: usize,
) {
    if let Some(&idx) = node_map.get(&node_id) {
        nodes[idx].signals.insert(signal_id);
        return;
    }

    let gnode = &fg.nodes[node_id];
    let gcost = &fg.costs[node_id];

    nodes.push(JsonNode {
        id: node_id,
        label: gnode.id.clone(),
        usage: gcost.usage,
        signals: HashSet::from([signal_id]),
    });

    node_map.insert(node_id, nodes.len() - 1);
}

/// Adds or updates a JSON edge
fn add_json_edge(
    fg: &FabricGraph,
    edges: &mut Vec<JsonEdge>,
    edge_map: &mut HashMap<(usize, usize), usize>,
    from: usize,
    to: usize,
    signal_id: usize,
) {
    let key = (from, to);

    if let Some(&idx) = edge_map.get(&key) {
        edges[idx].signals.insert(signal_id);
        return;
    }

    let weight = fg.map[from]
        .iter()
        .find_map(|e| if e.node_id == to { Some(e.cost) } else { None })
        .unwrap_or(0.0);

    edges.push(JsonEdge {
        from,
        to,
        weight,
        signals: HashSet::from([signal_id]),
    });

    edge_map.insert(key, edges.len() - 1);
}
