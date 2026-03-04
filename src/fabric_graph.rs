//! Module `fabric_graph`
//!
//! This module defines the FPGA fabric graph, routing paths, and related
//! operations including reading from a file, generating routing plans,
//! and computing distances and reversed maps.

use serde::{Deserialize, Serialize};

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader},
};

use crate::{
    FabricError, FabricResult,
    node::{Costs, Edge, Node},
};
/// Routing request from a source to multiple sinks
#[derive(Debug, Clone)]
pub struct Routing {
    /// Destination node indices
    pub sinks: Vec<usize>,
    /// Source signal node
    pub signal: usize,
    /// Optional routing result after computation
    pub result: Option<RoutingResult>,
    pub steiner_tree: Option<SteinerTree>,
}

impl Routing {
    pub fn expand(&self, graph: &FabricGraph) -> RoutingExpanded {
        let signal = graph.nodes[self.signal].id();
        let sinks = self.sinks.iter().map(|a| graph.nodes[*a].id()).collect();
        let result = self.result.as_ref().map(|r| r.expand(graph));

        RoutingExpanded { sinks, signal, result }
    }

    pub fn from_expanded(expanded: &RoutingExpanded, graph: &FabricGraph) -> Result<Self, String> {
        let mut signal: Option<usize> = None;
        let mut sinks_cloned = expanded.sinks.iter().cloned().collect::<HashSet<String>>();
        let mut sinks: Vec<usize> = vec![];

        for (i, node) in graph.nodes.iter().enumerate() {
            let id = node.id();
            if id == expanded.signal {
                signal = Some(i);
            }
            if sinks_cloned.remove(&id) {
                sinks.push(i);
            }
        }
        match (signal, sinks_cloned.is_empty()) {
            (Some(signal), true) => Ok(Self {
                sinks,
                signal,
                result: None,
                steiner_tree: None,
            }),
            (Some(_), false) => Err(format!("Sinks: {sinks_cloned:?} do not exist.")),
            (None, true) => Err("Signal does not exist in graph".to_string()),
            (None, false) => Err(format!("Signal does not exist and sinks: {sinks_cloned:?}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SteinerTree {
    /// This defines a Steiner Tree.
    /// It maps a terminal to the steiner nodes it needs to go to
    /// aswell as at the end the sink.
    pub steiner_nodes: HashMap<usize, Vec<usize>>,
    pub nodes: HashSet<usize>,
}

pub struct SteinerTreeCandidate {
    pub steiner_nodes: HashMap<usize, Vec<usize>>,
    pub nodes: HashSet<usize>,
    pub costs: f32,
}

/// Routing result for a routing request
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// Paths from source to each sink
    pub paths: HashMap<usize, Vec<usize>>,
    /// All nodes used in the routing
    pub nodes: HashSet<usize>,
}

impl RoutingResult {
    pub fn expand(&self, graph: &FabricGraph) -> RoutingResultExpanded {
        let nodes = self.nodes.iter().map(|a| graph.nodes[*a].id()).collect::<HashSet<String>>();
        let paths = self
            .paths
            .iter()
            .map(|(sink, path)| {
                (
                    graph.nodes[*sink].id(),
                    path.iter().map(|c| graph.nodes[*c].id()).collect::<Vec<String>>(),
                )
            })
            .collect::<HashMap<String, Vec<String>>>();

        RoutingResultExpanded { paths, nodes }
    }
}

/// Representation of the FPGA fabric graph
#[derive(Debug, Clone)]
pub struct FabricGraph {
    /// List of nodes in the graph
    pub nodes: Vec<Node>,
    /// Costs associated with each node
    pub costs: Vec<Costs>,
    /// Forward adjacency list
    pub map: Vec<Vec<Edge>>,
    /// Reversed adjacency list
    pub map_reversed: Vec<Vec<Edge>>,
}

impl FabricGraph {
    pub fn from_file(path: &str) -> FabricResult<Self> {
        let file = File::open(path).map_err(|e| FabricError::Io {
            path: path.to_string(),
            source: e,
        })?;
        let reader = BufReader::new(file);

        let mut nodes: Vec<Node> = vec![];
        let mut costs: Vec<Costs> = vec![];
        let mut map: Vec<Vec<Edge>> = Vec::new();
        let mut index: HashMap<Node, usize> = HashMap::new();

        for (line_number, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| FabricError::Io {
                path: path.to_string(),
                source: e,
            })?;

            let line = line.trim();
            // skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (start, end) = Node::parse_from_pips_line(line).map_err(|e| FabricError::LineError {
                line_number,
                content: line.to_string(),
                source: e,
            })?;

            // get or insert start
            let sid = *index.entry(start.clone()).or_insert_with(|| {
                nodes.push(start.clone());
                costs.push(Costs::new());
                map.push(Vec::new());
                nodes.len() - 1
            });

            // get or insert end
            let eid = *index.entry(end.clone()).or_insert_with(|| {
                nodes.push(end.clone());
                costs.push(Costs::new());
                map.push(Vec::new());
                nodes.len() - 1
            });

            let cost = Self::distance(&start, &end);
            map[sid].push(Edge { node_id: eid, cost });
        }
        let reversed = get_reversed_map(&nodes, &map);

        Ok(Self {
            nodes,
            costs,
            map,
            map_reversed: reversed,
        })
    }
    pub fn route_plan_expanded_form_file(file: &str) -> Result<Vec<RoutingExpanded>, FabricError> {
        let data: String = fs::read_to_string(file).map_err(|e| FabricError::Io {
            path: file.to_string(),
            source: e,
        })?;
        let r: Vec<RoutingExpanded> = serde_json::de::from_str(&data)?;
        Ok(r)
    }

    pub fn route_plan_form_file(&self, file: &str) -> Result<Vec<Routing>, FabricError> {
        let r = Self::route_plan_expanded_form_file(file)?;
        let r = r
            .into_iter()
            .map(|a| {
                Routing::from_expanded(&a, self).map_err(|e| FabricError::MappingExpandedRoutePlan {
                    signal: a.signal,
                    reason: e,
                })
            })
            .collect::<Result<Vec<Routing>, FabricError>>()?;
        Ok(r)
    }

    /// Distance function between nodes (Manhatten Distance)
    /// Will be our base costs
    const fn distance(a: &Node, b: &Node) -> f32 {
        (1 + a.x.abs_diff(b.x) + a.y.abs_diff(b.y)) as f32
    }

    pub fn reset_usage(&mut self) {
        self.costs.iter_mut().for_each(|a| a.usage = 0);
    }
}

/// Generate reversed adjacency list from forward map
fn get_reversed_map(nodes: &[Node], map: &[Vec<Edge>]) -> Vec<Vec<Edge>> {
    let n = nodes.len();
    let mut rev_map = vec![Vec::new(); n];

    for (u, edge_list) in map.iter().enumerate() {
        for edge in edge_list {
            let v = edge.node_id;

            rev_map[v].push(Edge {
                node_id: u,
                cost: edge.cost,
            });
        }
    }

    rev_map
}

/// Represents a entry in the `NetList`
/// each net has a start point (signal) and endpoints (sinks)
/// result contains the paths of the signal to each sink
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingExpanded {
    /// Destination node indices
    pub sinks: Vec<String>,
    /// Source signal node
    pub signal: String,
    /// Optional routing result after computation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<RoutingResultExpanded>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResultExpanded {
    /// Paths from source to each sink
    pub paths: HashMap<String, Vec<String>>,
    /// All nodes used in the routing
    pub nodes: HashSet<String>,
}

pub fn bucket_luts(nodes: &[Node]) -> (Vec<usize>, Vec<usize>) {
    let mut lut_inputs = vec![];
    let mut lut_outputs = vec![];
    for (i, node) in nodes.iter().enumerate() {
        if node.id.starts_with('L') {
            if node.id.chars().nth(3) == Some('O') {
                lut_outputs.push(i);
            } else if node.id.chars().nth(3) == Some('I') {
                lut_inputs.push(i);
            }
        }
    }
    (lut_inputs, lut_outputs)
}
