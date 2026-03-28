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
    FabricError, FabricResult, NetInternal, NetListInternal, SlackReport, graph::{
        node::{Costs, Edge, Node, NodeId, from_str_coords},
        parser::{Parser, TimingModel},
    }
};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    High,
    Low,
}
#[derive(Debug)]
pub enum LutState {
    Free,
    Used,
    Borrowed(State),
}

#[derive(Debug)]
pub struct Lut {
    bel_index: char,
    state: LutState,
    output_pin: String,
    _input_pin: [String; 4],
}
#[derive(Debug)]
pub struct Tile {
    id: TileId,
    luts: Vec<Lut>,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct TileId(pub u8, pub u8);

#[derive(Debug)]
pub struct TileManager(pub HashMap<TileId, Tile>);

impl TileManager {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let mut tiles: HashMap<TileId, Tile> = HashMap::new();

        for line in reader.lines() {
            let line = line.unwrap();
            // Skip comments and empty lines
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split(',').collect();

            // Basic validation for the FABULOUS_LC rows
            if parts.len() < 13 || parts[4] != "FABULOUS_LC" {
                continue;
            }

            // Parse Coordinates: Expecting "X1Y1" format in parts[0]
            // Or use parts[1] and parts[2] if they are raw integers

            let (x, y) = from_str_coords(parts[0]).unwrap();
            let tile_id = TileId(x, y);

            // Construct the LUT
            let lut = Lut {
                // parts[3] is "A", "B", etc.
                bel_index: parts[3].chars().next().unwrap_or('?'),
                state: LutState::Free,
                // parts[12] is the output pin (e.g., "LA_O")
                output_pin: parts[12].to_string(),
                // parts[5..9] are I0, I1, I2, I3
                _input_pin: [
                    parts[5].to_string(),
                    parts[6].to_string(),
                    parts[7].to_string(),
                    parts[8].to_string(),
                ],
            };

            // Insert into the tile manager
            tiles
                .entry(tile_id)
                .or_insert_with(|| Tile {
                    id: tile_id,
                    luts: Vec::new(),
                })
                .luts
                .push(lut);
        }

        Self(tiles)
    }
    /// Internal helper to find a LUT by index within a specific tile
    fn find_lut_mut(&mut self, tile_id: TileId, bel_index: char) -> Option<&mut Lut> {
        self.0
            .get_mut(&tile_id)
            .and_then(|tile| tile.luts.iter_mut().find(|lut| lut.bel_index == bel_index))
    }

    /// Marks a LUT as 'Used' (called during placement parsing)
    pub fn mark_lut_used(&mut self, tile: TileId, bel_index: char) -> Option<String> {
        if let Some(lut) = self.find_lut_mut(tile, bel_index) {
            // Safety check: only borrow if it's actually free
            if matches!(lut.state, LutState::Free) {
                lut.state = LutState::Used;
                return Some(lut.output_pin.clone());
            }
        }
        None
    }

    pub fn request_constant(&mut self, start_tile: TileId, state: State) -> Option<(TileId, String)> {
        // 1. Define the search radius (Starting Tile, then Neighbors)
        let search_order = [
            start_tile,
            TileId(start_tile.0 + 1, start_tile.1), // East
            TileId(start_tile.0, start_tile.1 + 1), // North
                                                    // ... add more as needed
        ];

        for &tid in &search_order {
            if let Some(tile) = self.0.get_mut(&tid) {
                // STEP A: Check if this tile ALREADY has a LUT borrowed for this state
                // This implements your "If it matches, use that" logic
                let existing = tile
                    .luts
                    .iter()
                    .find(|l| matches!(&l.state, LutState::Borrowed(s) if s == &state));

                if let Some(lut) = existing {
                    return Some((tile.id, lut.output_pin.clone()));
                }

                // STEP B: If no existing match, find the first FREE lut in this tile to borrow
                let free_lut_index = tile.luts.iter().position(|l| matches!(l.state, LutState::Free));

                if let Some(idx) = free_lut_index {
                    let lut = &mut tile.luts[idx];
                    lut.state = LutState::Borrowed(state);
                    return Some((tile.id, lut.output_pin.clone()));
                }
            }
        }
        None
    }
    /// Iterates through all tiles and generates FASM configuration strings
    /// for LUTs that were borrowed as constant drivers.
    pub fn generate_constant_fasm(&self) -> Vec<String> {
        let mut fasm_lines = Vec::new();

        for (tile_id, tile) in &self.0 {
            for lut in &tile.luts {
                if let LutState::Borrowed(state) = &lut.state {
                    // Example FASM Format: Tile_X1Y1.LC_A.INIT[15:0] = 16'h0000
                    let init_val = match state {
                        State::Low => "16'b0000000000000000",
                        State::High => "16'h1111111111111111",
                    };

                    // We use the bel_index (e.g., 'A', 'B') to specify which LUT in the tile
                    let line = format!("X{}Y{}.{}.INIT[15:0] = {}", tile_id.0, tile_id.1, lut.bel_index, init_val);

                    fasm_lines.push(line);
                }
            }
        }
        fasm_lines
    }
}

impl Fabric {
    pub fn new(graph: FabricGraph, tile_manager: TileManager) -> Self {
        Self { tile_manager, graph , slack_report: None}
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
            let state = state.ok_or(FabricError::Other(format!(
                "Cannot find a routing for net {} -> {}",
                sink_node.id(),
                signal_node.id()
            )))?;
            let tile = TileId(sink_node.x, sink_node.y);
            let new_source_name = self
                .tile_manager
                .request_constant(tile, state)
                .ok_or(FabricError::Other("Fabric exhausted: No free LUTs for constants".into()))?;

            let node_id_str = format!("X{}Y{}.{}", new_source_name.0.0, new_source_name.0.1, new_source_name.1);
            let node = self.graph.get_node_id(&node_id_str).unwrap();
            // 4. Re-run Dijkstra with the new local source
            let _ = self.graph.dijkstra(*node, *sink, 0.0).ok_or(FabricError::Other(format!(
                "Even local constant {:?} couldn't reach sink",
                new_source_name
            )))?;
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
    pub fn check_and_mark_node(&mut self, node_id: NodeId) {
        let node = self.graph.get_node(node_id);

        // FABulous naming convention: LA_I0, LB_O, LC_EN...
        // They all start with 'L' and a char [A-H], then an underscore
        if node.id.starts_with('L')
            && node.id.chars().nth(2) == Some('_')
            && let Some(bel_char) = node.id.chars().nth(1)
        {
            let tile_id = TileId(node.x, node.y);
            self.tile_manager.mark_lut_used(tile_id, bel_char);
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
    /// let graph = FabricGraph::from_file(test_file).unwrap();
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P, timing_model: Option<TimingModel>) -> FabricResult<Self> {
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
    use super::*;
    use testing_utils::get_test_data_path;
    #[test]
    fn test_parse_pips_file() {
        let test_file = get_test_data_path("pips_8x8.txt");
        let timing_model = TimingModel::default();

        let graph = FabricGraph::from_file(test_file, Some(timing_model)).unwrap();
        assert_eq!(graph.nodes[0], Node::parse("N1END3", "X1Y0").unwrap());
    }
    #[test]
    fn test_parse_pips_file_error_accessing_file() {
        let test_file = "some_file_that_does_not_exist.txt";
        let error = FabricGraph::from_file(test_file, None).unwrap_err().to_string();
        assert_eq!("IO error while accessing 'some_file_that_does_not_exist.txt'", error);
    }
    #[test]
    fn test_parse_bels_file() {
        let test_file = get_test_data_path("bel.txt");
        let _ = TileManager::from_file(test_file);
    }
    #[test]
    fn test_mark_used() {
        let test_file = get_test_data_path("bel.txt");
        let mut tile_manager = TileManager::from_file(test_file);
        let _ = tile_manager.mark_lut_used(TileId(1, 1), 'A').unwrap();
    }
    #[test]
    fn test_mark_borrowed() {
        let test_file = get_test_data_path("bel.txt");
        let mut tile_manager = TileManager::from_file(test_file);
        let _ = tile_manager.request_constant(TileId(1, 1), State::High).unwrap();
    }
}
