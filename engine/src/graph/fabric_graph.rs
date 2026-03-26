//! Module `fabric_graph`
//!
//! This module defines the FPGA fabric graph, routing paths, and related
//! operations including reading from a file, generating routing plans,
//! and computing distances and reversed maps.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{
    FabricError, FabricResult,
    graph::{node::{Costs, Edge, Node, NodeId}, parser::Parser},
};

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
    pub fn from_file<P: AsRef<Path>>(path: P) -> FabricResult<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| FabricError::Io {
            path: path_ref.to_path_buf(),
            source: e,
        })?;
        let mut pips_parser = Parser::new();
        let reader = BufReader::new(file);

        let reader = reader.lines().enumerate();
        for (line_number, line) in reader {
            let line = line.map_err(|e| FabricError::Io {
                path: path_ref.to_path_buf(),
                source: e,
            })?;
            pips_parser.parse_line(&line).map_err(|source| FabricError::ParseError {line_number, source})?;
        }
        Ok(pips_parser.build())
    }

    pub fn reset_usage(&mut self) {
        self.costs.iter_mut().for_each(|a| a.usage = 0);
    }

    pub(crate) fn get_node_id(&self, id: &str) -> Option<&NodeId> {
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

        let graph = FabricGraph::from_file(test_file).unwrap();
        assert_eq!(graph.nodes[0], Node::parse("N1END3", "X1Y0").unwrap());
    }
    #[test]
    fn test_parse_pips_file_error_accessing_file() {
        let test_file = "some_file_that_does_not_exist.txt";
        let error = FabricGraph::from_file(test_file).unwrap_err().to_string();
        assert_eq!(
            "IO error while accessing 'some_file_that_does_not_exist.txt'",
            error
        );
    }

}
