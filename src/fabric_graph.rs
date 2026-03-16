//! Module `fabric_graph`
//!
//! This module defines the FPGA fabric graph, routing paths, and related
//! operations including reading from a file, generating routing plans,
//! and computing distances and reversed maps.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    FabricError, FabricResult,
    error::ParseError,
    node::{Costs, Edge, Node, NodeId},
};

/// Representation of the FPGA fabric graph
#[derive(Debug, Clone, Default)]
pub struct FabricGraph {
    pub filename: String,
    /// List of nodes in the graph
    pub nodes: Vec<Node>,
    /// Costs associated with each node
    pub costs: Vec<Costs>,
    /// Forward adjacency list
    pub map: Vec<Vec<Edge>>,
    /// Reversed adjacency list
    pub map_reversed: Vec<Vec<Edge>>,
}

struct PipsParser {
    graph: FabricGraph,
    index: HashMap<Node, NodeId>,
}
struct PipsLine {
    start_node: Node,
    end_node: Node,
    _p1: String,
    _p2: String,
}

impl PipsParser {
    fn new() -> Self {
        Self {
            graph: FabricGraph::default(),
            index: HashMap::default(),
        }
    }
    fn parse_line(&mut self, line: &str, line_number: usize) -> FabricResult<()> {
        let line = line.trim();
        // skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        let PipsLine {
            start_node, end_node, ..
        } = parse_pips_line(line).map_err(|e| FabricError::LineError {
            line_number,
            content: line.to_string(),
            source: e,
        })?;

        let cost = distance(&start_node, &end_node);
        let sid = self.get_or_create_node(&start_node)?;
        let eid = self.get_or_create_node(&end_node)?;

        self.graph.map[sid as usize].push(Edge { node_id: eid, cost });
        Ok(())
    }

    fn get_or_create_node(&mut self, node: &Node) -> FabricResult<NodeId> {
        if let Some(sid) = self.index.get(node) {
            return Ok(*sid);
        }
        let id = NodeId::try_from(self.graph.nodes.len()).map_err(|_| FabricError::NodeIdValueSpaceTooSmall)?;
        self.index.insert(node.clone(), id);
        self.graph.nodes.push(node.clone());
        self.graph.costs.push(Costs::new());
        self.graph.map.push(Vec::new());
        Ok(id)
    }

    fn build_graph(mut self) -> FabricGraph {
        self.graph.map_reversed = get_reversed_map(&self.graph.nodes, &self.graph.map);
        self.graph
    }
}

impl FabricGraph {
    #[must_use]
    pub fn get_node(&self, node_id: NodeId) -> &Node {
        &self.nodes[node_id as usize]
    }
    #[must_use]
    pub fn get_costs(&self, node_id: NodeId) -> &Costs {
        &self.costs[node_id as usize]
    }
    pub fn get_costs_mut(&mut self, node_id: NodeId) -> &mut Costs {
        &mut self.costs[node_id as usize]
    }
    #[must_use]
    /// Returns the edge that connects `start` to `end`
    ///
    /// # Panics
    /// This panics when the graph does not contain that edge
    pub fn get_edge_panic(&self, start: NodeId, end: NodeId) -> &Edge {
        self.map[start as usize]
            .iter()
            .find(|a| a.node_id == end)
            .map_or_else(|| panic!("Graph did not contain the edge: a: {start}, b: {end}"), |edge| edge)
    }
    /// Returns the edge that connects `start` to `end`
    ///
    /// # Errors
    /// This fails when the graph does not contain that edge
    pub fn get_edge(&self, start: NodeId, end: NodeId) -> FabricResult<&Edge> {
        self.map[start as usize]
            .iter()
            .find(|a| a.node_id == end)
            .ok_or(FabricError::EdgeDoesNotExist { start, end })
    }

    /// Parses a `pips.txt` file to a `FabricGraph`
    ///
    /// # Errors
    /// This function fails when the provided file is invalid.
    ///
    /// # Example
    /// ```
    /// use router::FabricGraph;
    ///
    /// let test_file = "pips_8x8.txt";
    /// let graph = FabricGraph::from_file(test_file).unwrap();
    ///
    /// ```
    pub fn from_file(path: &str) -> FabricResult<Self> {
        let file = File::open(path).map_err(|e| FabricError::Io {
            path: path.to_string(),
            source: e,
        })?;
        let mut pips_parser = PipsParser::new();
        let reader = BufReader::new(file);

        let reader = reader.lines().enumerate();
        for (line_number, line) in reader {
            let line = line.map_err(|e| FabricError::Io {
                path: path.to_string(),
                source: e,
            })?;
            pips_parser.parse_line(&line, line_number)?;
        }
        Ok(pips_parser.build_graph())
    }

    pub fn reset_usage(&mut self) {
        self.costs.iter_mut().for_each(|a| a.usage = 0);
    }

    pub(crate) fn reset(&mut self){
        self.costs.iter_mut().for_each(|cost| {
            cost.historic_cost = 0.0;
            cost.usage = 0;
        })
    }
}

/// Generate reversed adjacency list from forward map
fn get_reversed_map(nodes: &[Node], map: &[Vec<Edge>]) -> Vec<Vec<Edge>> {
    let n = nodes.len();
    let mut rev_map = vec![Vec::new(); n];

    for (u, edge_list) in map.iter().enumerate() {
        for edge in edge_list {
            let v = edge.node_id;

            #[allow(clippy::cast_possible_truncation)]
            let node_id = u as NodeId;
            let cost = edge.cost;

            rev_map[v as usize].push(Edge { node_id, cost });
        }
    }

    rev_map
}

pub fn bucket_luts(nodes: &[Node]) -> (Vec<NodeId>, Vec<NodeId>) {
    let mut lut_inputs = vec![];
    let mut lut_outputs = vec![];
    for (i, node) in nodes.iter().enumerate() {
        if node.id.starts_with('L') {
            #[allow(clippy::cast_possible_truncation)]
            let id = i as NodeId;
            if node.id.chars().nth(3) == Some('O') {
                lut_outputs.push(id);
            } else if node.id.chars().nth(3) == Some('I') {
                lut_inputs.push(id);
            }
        }
    }
    (lut_inputs, lut_outputs)
}

fn parse_pips_line(line: &str) -> Result<PipsLine, ParseError> {
    if let [node1_cords, node1_id, node2_cords, node2_id, _, _] = line.split(',').collect::<Vec<&str>>().as_slice() {
        let start_node = Node::parse(node1_id, node1_cords).map_err(|e| ParseError::InvalidStartNode {
            id: (*node1_id).to_string(),
            cords: (*node1_cords).to_string(),
            source: e.into(),
        })?;
        let end_node = Node::parse(node2_id, node2_cords).map_err(|e| ParseError::InvalidEndNode {
            id: (*node2_id).to_string(),
            cords: (*node2_cords).to_string(),
            source: e.into(),
        })?;
        Ok(PipsLine {
            start_node,
            end_node,
            _p1: String::default(),
            _p2: String::default(),
        })
    } else {
        Err(ParseError::InvalidLineFormat)
    }
}
/// Distance function between nodes (Manhatten Distance)
/// Will be our base costs
const fn distance(a: &Node, b: &Node) -> f32 {
    (1 + a.x.abs_diff(b.x) + a.y.abs_diff(b.y)) as f32
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_pips_file() {
        let test_file = "pips_8x8.txt";
        let graph = FabricGraph::from_file(test_file).unwrap();
        assert_eq!(graph.nodes[0], Node::parse("N1END3", "X1Y0").unwrap());
    }
    #[test]
    fn test_parse_pips_file_error_accessing_file() {
        let test_file = "some_file_that_does_not_exist.txt";
        let error = FabricGraph::from_file(test_file).unwrap_err().to_string();
        assert_eq!(
            "IO error while accessing 'some_file_that_does_not_exist.txt': No such file or directory (os error 2)",
            error
        );
    }

    #[test]
    fn test_parse_from_pips_line_success() {
        let test_case = "X1Y0,N1END3,X1Y0,S1BEG0,8,N1END3.S1BEG0".to_string();
        let node1_expected = Node {
            id: "N1END3".to_string(),
            x: 1,
            y: 0,
        };
        let node2_expected = Node {
            id: "S1BEG0".to_string(),
            x: 1,
            y: 0,
        };
        let PipsLine {
            start_node, end_node, ..
        } = parse_pips_line(&test_case).unwrap();
        assert_eq!(start_node, node1_expected);
        assert_eq!(end_node, node2_expected);
    }
    #[test]
    fn test_parse_from_pips_line_failure_line_format() {
        let test_case = "X1Y0,,N1END3,X1Y0,S1BEG0,8,N1END3.S1BEG0".to_string();
        let error_message = "Wrong Pips line format. Expecting 6 parts.".to_string();
        if let Err(result) = parse_pips_line(&test_case) {
            assert_eq!(error_message, result.to_string());
        } else {
            panic!("This should return an error!");
        }
    }
    #[test]
    fn test_parse_from_pips_line_failure_start_node() {
        let test_case = "X1Yp,N1END3,X1Y0,S1BEG0,8,N1END3.S1BEG0".to_string();
        let error_message = "Failed to parse start node id: N1END3 cords: X1Yp".to_string();
        if let Err(result) = parse_pips_line(&test_case) {
            assert_eq!(error_message, result.to_string());
        } else {
            panic!("This should return an error!");
        }
    }
    #[test]
    fn test_parse_from_pips_line_failure_end_node() {
        let test_case = "X1Y1,N1END3,X1Yp,S1BEG0,8,N1END3.S1BEG0".to_string();
        let error_message = "Failed to parse end node id: S1BEG0 cords: X1Yp".to_string();
        if let Err(result) = parse_pips_line(&test_case) {
            assert_eq!(error_message, result.to_string());
        } else {
            panic!("This should return an error!");
        }
    }
}
