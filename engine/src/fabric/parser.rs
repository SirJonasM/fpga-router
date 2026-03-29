use crate::{
    FabricGraph,
    fabric::{
        error::ParseError,
        node::{Costs, Edge, Node, NodeId},
    },
};

struct PipsLine {
    start_node: Node,
    end_node: Node,
    _p1: String,
    _p2: String,
}

pub struct Parser {
    graph: FabricGraph,
    timing_model: Option<TimingModel>,
}

#[derive(Debug, Default)]
pub struct TimingModel {
    pub lut_delay: f64,
    pub pip_delay: f64,
    pub fanout_delay: f64,
    pub clock_to_output_delay: f64,
    pub clock_tree_delay: f64,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            graph: FabricGraph::default(),
            timing_model: None,
        }
    }
    pub const fn set_timing_model(&mut self, timing_model: TimingModel) {
        self.timing_model = Some(timing_model);
    }
    pub fn parse_line(&mut self, line: &str) -> Result<(), ParseError> {
        let line = line.trim();
        // skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        let PipsLine {
            start_node, end_node, ..
        } = parse_pips_line(line).map_err(|e| ParseError::LineError {
            content: line.to_string(),
            source: Box::new(e),
        })?;

        #[allow(clippy::cast_possible_truncation)]
        let cost = self
            .timing_model
            .as_ref()
            .map_or_else(|| distance(&start_node, &end_node), |a| a.pip_delay as f32);

        let sid = self.get_or_create_node(&start_node);
        let eid = self.get_or_create_node(&end_node);

        self.graph.map[sid].push(Edge { node_id: eid, cost });
        self.graph.map_reversed[eid].push(Edge { node_id: sid, cost });

        Ok(())
    }

    fn get_or_create_node(&mut self, node: &Node) -> NodeId {
        if let Some(sid) = self.graph.index.get(&node.id()) {
            return *sid;
        }
        let id = NodeId::new(self.graph.nodes.len());
        self.graph.index.insert(node.id(), id);
        self.graph.nodes.push(node.clone());
        self.graph.costs.push(Costs::new());

        self.graph.map.push(Vec::new());
        self.graph.map_reversed.push(Vec::new());

        id
    }
    pub fn build(self) -> FabricGraph {
        self.graph
    }
}

fn parse_pips_line(line: &str) -> Result<PipsLine, ParseError> {
    if let [node1_cords, node1_id, node2_cords, node2_id, _, _] = line.split(',').collect::<Vec<&str>>().as_slice() {
        let start_node = Node::parse(node1_id, node1_cords).map_err(|e: ParseError| ParseError::InvalidStartNode {
            id: (*node1_id).to_string(),
            cords: (*node1_cords).to_string(),
            source: e.into(),
        })?;
        let end_node = Node::parse(node2_id, node2_cords).map_err(|e: ParseError| ParseError::InvalidEndNode {
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
    (1 + a.tile.0.abs_diff(b.tile.0) + a.tile.1.abs_diff(b.tile.1)) as f32
}

#[cfg(test)]
mod test {
    use crate::fabric::node::TileId;

    use super::*;

    #[test]
    fn test_parse_from_pips_line_success() {
        let test_case = "X1Y0,N1END3,X1Y0,S1BEG0,8,N1END3.S1BEG0".to_string();
        let node1_expected = Node {
            id: "N1END3".to_string(),
            tile: TileId(1,0)
        };
        let node2_expected = Node {
            id: "S1BEG0".to_string(),
            tile: TileId(1,0)
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
