//! Module `fabric_graph`
//!
//! This module defines the FPGA fabric graph, routing paths, and related
//! operations including reading from a file, generating routing plans,
//! and computing distances and reversed maps.

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{
    FabricError, FabricResult, NetInternal, NetListInternal, SlackReport,
    fabric::{
        node::{Costs, Edge, Node, NodeId},
        parser::{Parser, TimingModel}, tile_manager::{State, TileManager, },
    },
};


impl Fabric {
    #[must_use]
    pub const fn new(graph: FabricGraph, tile_manager: TileManager) -> Self {
        Self {
            tile_manager,
            graph,
            slack_report: None,
        }
    }

    pub(crate) fn check_pathing(&mut self, net_list: &mut NetListInternal) -> FabricResult<()> {
        let net_list_flatten = net_list
            .plan
            .iter()
            .flat_map(|a| a.sinks.iter().map(|v| (a.signal, *v)))
            .collect::<HashSet<(NodeId, NodeId)>>();

        for (signal, sink) in &net_list_flatten {
            // Check the Source
            self.check_and_mark_node(*signal);
            self.check_and_mark_node(*sink);
        }

        let mut optimized_net = HashSet::new();
        for (signal, sink) in &net_list_flatten {
            let signal_node = self.graph.get_node(*signal);
            if self.graph.dijkstra(*signal, *sink, 0.0).is_some() {
                optimized_net.insert((*signal, *sink));
                continue;
            }
            let state = match signal_node.id.as_str() {
                "VCC0" => Some(State::High),
                "GND0" => Some(State::Low),
                _ => None,
            };
            let sink_node = self.graph.get_node(*sink);
            let state = state.ok_or_else(|| {
                FabricError::Other(format!(
                    "Cannot find a routing for net {} -> {}",
                    sink_node.id(),
                    signal_node.id()
                ))
            })?;
            let tile = sink_node.tile;
            let new_source_name = self
                .tile_manager
                .request_constant(tile, state)
                .ok_or_else(|| FabricError::Other("Fabric exhausted: No free LUTs for constants".into()))?;

            let node_id_str = format!("X{}Y{}.{}", new_source_name.0.0, new_source_name.0.1, new_source_name.1);
            let node = self.graph.get_node_id(&node_id_str).unwrap();
            // 4. Re-run Dijkstra with the new local source
            let _ = self
                .graph
                .dijkstra(*node, *sink, 0.0)
                .ok_or_else(|| FabricError::Other(format!("Even local constant {new_source_name:?} couldn't reach sink")))?;
            optimized_net.insert((*node, *sink));
        }

        // 1. Group sinks by their signal (source)
        let mut grouped_nets: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for (signal, sink) in optimized_net {
            grouped_nets.entry(signal).or_default().push(sink);
        }

        // 2. Map the groups back into NetInternal structures
        let new_plan: Vec<NetInternal> = grouped_nets
            .into_iter()
            .map(|(signal, sinks)| NetInternal {
                signal,
                sinks,
                result: None,
                intermediate_nodes: None,
                priority: None,    // Set to default as requested
                criticallity: 0.0, // Set to default as requested
            })
            .collect();
        *net_list = NetListInternal { plan: new_plan };
        Ok(())
    }
}

pub struct Fabric {
    pub tile_manager: TileManager,
    pub graph: FabricGraph,
    pub slack_report: Option<SlackReport>,
}

impl Fabric {
#[allow(clippy::missing_panics_doc)]
    pub fn check_and_mark_node(&mut self, node_id: NodeId) {
        let node = self.graph.get_node(node_id);

        // FABulous naming convention: LA_I0, LB_O, LC_EN...
        // They all start with 'L' and a char [A-H], then an underscore
        if node.id.starts_with('L')
            && node.id.chars().nth(2) == Some('_')
            && let Some(bel_char) = node.id.chars().nth(1)
        {
            self.tile_manager.mark_lut_used(node.tile, bel_char);
            if node.id.chars().nth(3) == Some('I'){
                self.tile_manager.mark_lut_input_used(node.tile, bel_char, &node.id).unwrap();
            }
        }
    }
}

/// Representation of the FPGA fabric graph
#[derive(Debug, Clone, Default)]
pub struct FabricGraph {
    pub nodes: Vec<Node>,
    /// Costs associated with each node
    pub costs: Vec<Costs>,
    /// Forward adjacency list
    pub map: Vec<Vec<Edge>>,
    /// Reversed adjacency list
    pub map_reversed: Vec<Vec<Edge>>,
    /// Index of String ids from PIPS file to internal `NodeId`
    pub index: HashMap<String, NodeId>,
}

impl FabricGraph {
    #[must_use]
    pub fn get_node(&self, node_id: NodeId) -> &Node {
        &self.nodes[node_id]
    }
    #[must_use]
    pub fn get_costs(&self, node_id: NodeId) -> &Costs {
        &self.costs[node_id]
    }
    pub fn get_costs_mut(&mut self, node_id: NodeId) -> &mut Costs {
        &mut self.costs[node_id]
    }
    #[must_use]
    /// Returns the edge that connects `start` to `end`
    ///
    /// # Panics
    /// This panics when the graph does not contain that edge
    pub fn get_edge_panic(&self, start: NodeId, end: NodeId) -> &Edge {
        self.map[start].iter().find(|a| a.node_id == end).unwrap_or_else(|| {
            panic!(
                "Graph did not contain the edge: a: {}, b: {}",
                start.name(self),
                end.name(self)
            )
        })
    }

    /// Returns the edge that connects `start` to `end`
    ///
    /// # Errors
    /// This fails when the graph does not contain that edge
    pub fn get_edge(&self, start: NodeId, end: NodeId) -> FabricResult<&Edge> {
        self.map[start]
            .iter()
            .find(|a| a.node_id == end)
            .ok_or_else(|| FabricError::EdgeDoesNotExist {
                start: start.name(self),
                end: end.name(self),
            })
    }

    /// Parses a `pips.txt` file to a `FabricGraph`
    ///
    /// # Errors
    /// This function fails when the provided file is invalid.
    ///
    /// # Example
    /// ```
    /// use testing_utils::get_test_data_path;
    /// use router::FabricGraph;
    ///
    /// let test_file = get_test_data_path("pips_8x8.txt");
    /// let graph = FabricGraph::from_file(&test_file, None).unwrap();
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: &P, timing_model: Option<TimingModel>) -> FabricResult<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| FabricError::Io {
            path: path_ref.to_path_buf(),
            source: e,
        })?;
        let mut pips_parser = Parser::new();
        if let Some(timing_model) = timing_model {
            pips_parser.set_timing_model(timing_model);
        }
        let reader = BufReader::new(file);

        let reader = reader.lines().enumerate();
        for (line_number, line) in reader {
            let line = line.map_err(|e| FabricError::Io {
                path: path_ref.to_path_buf(),
                source: e,
            })?;
            pips_parser
                .parse_line(&line)
                .map_err(|source| FabricError::ParseError { line_number, source })?;
        }
        Ok(pips_parser.build())
    }

    pub fn reset_usage(&mut self) {
        self.costs.iter_mut().for_each(|a| a.usage = 0);
    }

    #[must_use]
    pub fn get_node_id(&self, id: &str) -> Option<&NodeId> {
        self.index.get(id)
    }

    #[must_use]
    pub fn calculate_structure_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // 1. Hash the number of nodes to start
        hasher.update((self.nodes.len() as u64).to_le_bytes());

        // 2. Hash node identifiers (assuming node.id() returns a string/bytes)
        for node in &self.nodes {
            hasher.update(node.id().as_bytes());
        }

        // 3. Hash the Adjacency Map
        for edge_list in &self.map {
            // Hash the length of the sub-vector to distinguish [[1], [2]] from [[1, 2]]
            hasher.update((edge_list.len() as u64).to_le_bytes());
            for edge in edge_list {
                // edge.node_id is our Newtype NodeId(u16)
                hasher.update(edge.node_id.0.to_le_bytes());
                // If cost is f32/f64, use to_bits() to get stable bytes
                hasher.update(edge.cost.to_bits().to_le_bytes());
            }
        }

        format!("{:x}", hasher.finalize())
    }
}

/// Generate reversed adjacency list from forward map
pub fn bucket_luts(graph: &FabricGraph) -> (Vec<NodeId>, Vec<NodeId>) {
    let mut lut_inputs = vec![];
    let mut lut_outputs = vec![];
    for node in &graph.nodes {
        if node.id.starts_with('L') {
            let id = *graph.get_node_id(&node.id()).unwrap();
            if node.id.chars().nth(3) == Some('O') {
                lut_outputs.push(id);
            } else if node.id.chars().nth(3) == Some('I') {
                lut_inputs.push(id);
            }
        }
    }
    (lut_inputs, lut_outputs)
}

#[cfg(test)]
mod test {
    use crate::fabric::node::TileId;

    use super::*;
    use testing_utils::get_test_data_path;
    #[test]
    fn test_parse_pips_file() {
        let test_file = get_test_data_path("pips_8x8.txt");
        let timing_model = TimingModel::default();

        let graph = FabricGraph::from_file(&test_file, Some(timing_model)).unwrap();
        assert_eq!(graph.nodes[0], Node::parse("N1END3", "X1Y0").unwrap());
    }
    #[test]
    fn test_parse_pips_file_error_accessing_file() {
        let test_file = "some_file_that_does_not_exist.txt";
        let error = FabricGraph::from_file(&test_file, None).unwrap_err().to_string();
        assert_eq!("IO error while accessing 'some_file_that_does_not_exist.txt'", error);
    }
    #[test]
    fn test_parse_bels_file() {
        let test_file = get_test_data_path("bel.txt");
        let _ = TileManager::from_file(&test_file);
    }
    #[test]
    fn test_mark_used() {
        let test_file = get_test_data_path("bel.txt");
        let mut tile_manager = TileManager::from_file(&test_file).unwrap();
        let _ = tile_manager.mark_lut_used(TileId(1, 1), 'A').unwrap();
    }
    #[test]
    fn test_mark_borrowed() {
        let test_file = get_test_data_path("bel.txt");
        let mut tile_manager = TileManager::from_file(&test_file).unwrap();
        let _ = tile_manager.request_constant(TileId(1, 1), State::High).unwrap();
    }
}
