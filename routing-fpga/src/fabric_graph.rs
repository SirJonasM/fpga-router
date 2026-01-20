//! Module `fabric_graph`
//!
//! This module defines the FPGA fabric graph, routing paths, and related
//! operations including reading from a file, generating routing plans,
//! and computing distances and reversed maps.

use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    SEED,
    node::{Costs, Edge, GraphNode, Node, NodeType},
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
    pub steiner_tree: Option<SteinerTree>
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

/// Representation of the FPGA fabric graph
#[derive(Debug, Clone)]
pub struct FabricGraph {
    /// LUT input node indices
    pub lut_inputs: Vec<usize>,
    /// LUT output node indices
    pub lut_outputs: Vec<usize>,
    /// Map from Node to index
    pub index: HashMap<Node, usize>,
    /// Costs associated with each node
    pub costs: Vec<Costs>,
    /// List of nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// Forward adjacency list
    pub map: Vec<Vec<Edge>>,
    /// Reversed adjacency list
    pub map_reversed: Vec<Vec<Edge>>,
}

impl FabricGraph {
    /// Build a FabricGraph from `pips.txt` file
    pub fn from_file(path: &str) -> Result<Self, String> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return Err(format!("Error loading file: {}.", path)),
        };
        let reader = BufReader::new(file);

        let mut nodes: Vec<GraphNode> = vec![];
        let mut costs: Vec<Costs> = vec![];
        let mut map: Vec<Vec<Edge>> = Vec::new();
        let mut index: HashMap<Node, usize> = HashMap::new();
        let mut lut_inputs = vec![];
        let mut lut_outputs = vec![];

        for line_result in reader.lines() {
            let line = match line_result{
                Ok(line) => line,
                Err(_err) => format!("Error reading line in file {}.", path),
            };

            let line = line.trim();
            // skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() != 6 {
                return Err(format!("Invalid line: {}", line));
            }

            let start = GraphNode::parse(parts[0], parts[1]);

            let end = GraphNode::parse(parts[2], parts[3]);

            // get or insert start
            let sid = *index.entry(start.node.clone()).or_insert_with(|| {
                nodes.push(start.clone());
                costs.push(Costs::new());
                map.push(Vec::new());
                nodes.len() - 1
            });

            // get or insert end
            let eid = *index.entry(end.node.clone()).or_insert_with(|| {
                nodes.push(end.clone());
                costs.push(Costs::new());
                map.push(Vec::new());
                nodes.len() - 1
            });

            let cost = Self::distance(&start.node, &end.node);
            map[sid].push(Edge { node_id: eid, cost });
        }
        for (i, node) in nodes.iter().enumerate() {
            match node.typ {
                NodeType::LutInput(_) => lut_inputs.push(i),
                NodeType::LutOutput(_) => lut_outputs.push(i),
                NodeType::Default(_) => {}
            }
        }
        let reversed = get_reversed_map(&nodes, &map);
        for &input_a in &lut_inputs {
            for &input_b in &lut_inputs {
                if input_a == input_b {
                    continue; // skip same index, or handle separately if needed
                }

                // Get two non-overlapping mutable references
                let (node_a, node_b) = if input_a < input_b {
                    let (left, right) = nodes.split_at_mut(input_b);
                    (&mut left[input_a], &mut right[0])
                } else {
                    let (left, right) = nodes.split_at_mut(input_a);
                    (&mut right[0], &mut left[input_b])
                };

                if node_a.node.x == node_b.node.x
                    && node_a.node.y == node_b.node.y
                    && let NodeType::LutInput(a) = &mut node_a.typ
                    && let NodeType::LutInput(b) = &mut node_b.typ
                    && a.letter == b.letter
                {
                    if !a.others.contains(&input_b) {
                        a.others.push(input_b);
                    }
                    if !b.others.contains(&input_a) {
                        b.others.push(input_a);
                    }
                }
            }
        }

        Ok(Self {
            index,
            nodes,
            costs,
            map,
            lut_inputs,
            lut_outputs,
            map_reversed: reversed,
        })
    }

    /// Generate a routing plan with a subset of outputs and inputs
    pub fn route_plan(&self, source_percentage: f32, output_count: usize) -> Vec<Routing> {
        let mut rng = StdRng::seed_from_u64(SEED);
        let mut sources = self.lut_outputs.clone();
        sources.shuffle(&mut rng);
        sources.truncate((source_percentage * sources.len() as f32).ceil() as usize);
        let mut destinations = self.lut_inputs.clone();
        destinations.shuffle(&mut rng);
        sources.truncate(sources.len() * output_count);
        let mut route_plan = vec![];
        while let Some(signal) = sources.pop() {
            let mut sinks = Vec::new();
            for _ in 0..output_count {
                if let Some(end) = destinations.pop() {
                    sinks.push(end)
                }
            }

            let routing = Routing {
                sinks,
                signal,
                result: None,
                steiner_tree: None,
            };
            route_plan.push(routing)
        }
        route_plan
    }

    /// Distance function between nodes (Manhatten Distance)
    /// Will be our base costs
    fn distance(a: &Node, b: &Node) -> f32 {
        (1 + a.x.abs_diff(b.x) + a.y.abs_diff(b.y)) as f32
    }

    pub fn reset_usage(&mut self) {
        self.costs.iter_mut().for_each(|a| a.usage = 0)
    }
}

/// Generate reversed adjacency list from forward map
fn get_reversed_map(nodes: &[GraphNode], map: &[Vec<Edge>]) -> Vec<Vec<Edge>> {
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
